# Ferrite v0.3.1 QA report — Linux

- Commit tested: `8e6ef508aef141b243b961e278861d76f8bc0d48` (`0.3.1-experimental`; commit date 2026-10-08 19:48 CEST). Binary reports `ferrite 0.3.1`; About shows "Version 0.3.1".
- Date / timezone: Thursday 2026-10-08, GUI testing 20:12–22:44 CEST (UTC+2). Times in this report are CEST unless they end in `Z`. Debug-log timestamps are UTC (`…Z`): CEST = Z + 2 h.
- Tier 2/3 follow-up: Batches 5–6 (Tier 2 rows 30–35 and 37–40, Tier 3 rows 41–47, §6 SI-1…5) were run Oct 8–9 2026, 22:55 CEST (Oct 8) to 00:06 CEST (Oct 9), on the same commit, binary and box. Raw results: `results-batch5.md`, `results-batch6.md`.
- Batches 7–12 (remaining §6 rows, total-test §1–§12, EXTRA-YT-7, EXTRA-SET-5/6/7/13/14): Oct 9 2026, 01:29–05:33 CEST, same commit, binary and box. Raw results: `results-batch7.md` … `results-batch12.md`.
- Distro, kernel, display server (X11/Wayland), GPU/software GL: Debian GNU/Linux 13 (trixie), kernel `6.12.94+`. **X11 only**: Xvfb (`xvfb 2:21.1.16-1.3+deb13u4`) display `:2`, 1920×1200×24, xfwm4 4.20.0, no compositor, no Wayland. **No GPU**: Mesa 25.0.7 **llvmpipe** (LLVM 19.1.7, 256 bits) software GL, OpenGL 4.5 core, eframe glow renderer. WebView stack: WebKitGTK 4.1 **2.54.0** (`libwebkit2gtk-4.1-0 2.54.0-1~deb13u2`) via wry.
- rustc version: `rustc 1.92.0 (ded5c06cf 2025-12-08)`, pinned by the repo's `rust-toolchain.toml` (`channel = "1.92.0"`).
- Build time (release): **273 s** (19:56:30 → 20:01 CEST; cargo: "Finished `release` profile [optimized] target(s) in 4m 33s").
- Tester: Grok Bot (autonomous agent) driving the GUI with synthetic X11 mouse and keyboard input. Every launch went through `run-ferrite.sh`, which runs `--log-level debug` and records the exit code (`0` clean, `134` panic/abort, `137` kill -9).
- Repo state at the end (`git -C /workspace/ferrite-qa/ferrite status --short`, re-checked by the report writer after Batch 12): empty, i.e. a clean tree at `8e6ef508`. The Batch-1 fixture `test_md/zz_cjk_search.md` and the Batch-12 scratch files (`test_md/zz_untracked.md`, the edit to `test_md/wikilink-target-a.md`) were removed or reverted by the testers. `logs/git-status-after-qa.txt` still shows the older end-of-Batch-6 state (`?? test_md/zz_cjk_search.md`). No tracked file was modified, and nothing was committed, pushed, or filed as an issue.

**Overall: NOT ready to tag.** All 12 batches are now in (Oct 8 20:12 CEST to Oct 9 05:33 CEST). Across the 198 counted rows: **83 PASS / 51 FAIL / 30 PARTIAL / 17 BLOCKED / 16 SKIPPED / 1 NOT RUN**. Tier 1 is unchanged: 13 PASS / 11 FAIL / 1 BLOCKED out of 25. Blockers, most serious first:
(1) **Rendered/Split self-deadlock (ISS-1).** Batches 9–12 hit it 10 more times, every time with the same gdb stack (now **22 freezes, 19 dumps**). New triggers are ordinary navigation: ArrowDown/ArrowUp in Rendered and in the Split preview (ARR-1, ARR-2, ARR-4), a drag-select in a paragraph's edit view (CPY-3), Tab on a list item in the preview (IND-9), and a plain click on a link inside a list item (UX-9). Any unsaved work is lost when the app has to be killed.
(2) **Any trusted YouTube embed aborts the app on Linux (ISS-2)**, `GDK has not been initialized`; EXTRA-YT-7 hit it twice more (9 launches in total).
(3) **Possible data loss after a session restore (ISS-36, seen once):** typing into a restored tab replaced the whole buffer with the typed text.
Then the must-pass Tier 1 failures 4b, 15 and 21 (ISS-3, ISS-4, ISS-7); Batch 7 showed the Search-in-Files caret reset (ISS-4) also for ASCII matches. The most important new **major** findings: with *Use system title bar* ON there is no way back into Settings and Ctrl+, does nothing (ISS-37); Exit → Save silently skips Untitled tabs (ISS-38); Rendered/preview edits reach the document late, so a save can miss a pending table-cell edit (ISS-41); copying from a code block inserts spaces (ISS-39); Rendered code-block typing/undo (ISS-40); Ctrl+click/middle-click links do nothing (ISS-42); terminal *Auto-load Layout* OFF is ignored (ISS-43). No new crash type was found; every abort is ISS-2 and every freeze is ISS-1.

| Tier / set | Rows | PASS | FAIL | PARTIAL | BLOCKED | SKIPPED | NOT RUN |
|---|---|---|---|---|---|---|---|
| Minimal Tier 1 (1–22 incl. 4b, 16b, 18b) | 25 | 13 | 11 | 0 | 1 | 0 | 0 |
| Minimal Tier 2 (23–40 incl. 25b) | 19 | 12 | 3 | 2 | 1 | 1 (row 40, platform) | 0 |
| Minimal Tier 3 (41–47) | 7 | 3 | 2 | 2 | 0 | 0 | 0 |
| Total test, Batch 4 (VID-1…9, VID-§9/§10/§11, VID-10, PLT-2, EOL-7) | 15 | 1 | 2 | 3 | 9 | 0 | 0 |
| Total test, Batch 9a (STAT-3, EOL-3/5/9, SES-1/3–6) | 9 | 5 | 3 | 1 | 0 | 0 | 0 |
| Total test, Batch 9b (ENT-6, FIND-5, ARR-1/2/4/5/6b, CPY-2/3, UND-3) | 10 | 4 | 6 | 0 | 0 | 0 | 0 |
| Total test, Batch 10 (RLD, IND, EMO, IMG) | 18 | 12 | 2 | 1 | 2 | 1 (IMG-2, no drag source) | 0 |
| Total test, Batch 11 (MMD) | 9 | 3 | 3 | 3 | 0 | 0 | 0 |
| Total test, Batch 12a (HTML, WS, CSV, TBL, UX) | 14 | 7 | 3 | 4 | 0 | 0 | 0 |
| Total test, Batch 12b (RUN, GIT, TTL-6a/b, MW-5) | 8 | 3 | 1 | 3 | 0 | 1 (MW-5, no v1 session fixture) | 0 |
| Total test, not run (platform skips TTL-1…5, MAC-1…4, PLT-1/3, RUN-2, GIT-1; optional STAT-4) | 14 | 0 | 0 | 0 | 0 | 13 (platform) | 1 |
| EXTRA-YT-1…8 (gui-test-plan) | 8 | 0 | 4 | 1 | 3 | 0 | 0 |
| EXTRA-SET-1…14 + EXTRA-SET-2-defect (gui-test-plan) | 15 | 6 | 2 | 7 | 0 | 0 | 0 |
| §6 extra rows (SIF, SI, PRI, SPL, FNT, IMG-X) | 27 | 14 | 9 | 3 | 1 | 0 | 0 |
| **All rows above** | **198** | **83** | **51** | **30** | **17** | **16** | **1** |

Total-test rows that are covered by a minimal or §6 row (e.g. FIND-1…4 → row 30, IMG-8 → IMG-X1) are listed in their own table below and are not counted again here. Row 39 is counted as PARTIAL (its Alt+Z sub-step is BLOCKED; everything else passed). Row 37 is counted as PASS; only its play-video sub-step was blocked by ISS-2. VID-10 and PLT-2 are now counted as their own rows; their verdicts are **derived** from VID-1, T2-36 and EXTRA-YT-2/4/6/7 and the release build, not from a separate run. TTL-6 is counted as two rows (TTL-6a custom title bar, TTL-6b system title bar). IMG-2 is SKIPPED because no drag source could be produced (input), not by platform. Severity for issues ISS-36 and later follows the brief's guide: blocker = crash, data loss, file corruption, cannot save or exit; major = feature does not work as described; minor = wrong but workable; polish = cosmetic.

### Environment caveats (read before triaging)

- **Software GL on a virtual framebuffer.** All rendering went through Mesa llvmpipe on Xvfb, with no GPU, no compositor, and no physical display. Some rendering findings may be partly environmental, especially the repaint lag where the UI updates only after the pointer moves (ISS-24) and any timing-sensitive behaviour. The deadlock (ISS-1) and the GDK panic (ISS-2) are **not** environmental. Both have deterministic stacks inside Ferrite code.
- **xfwm4 grabs Alt+click and Alt+Space.** `xfconf-query -c xfwm4 -p /general/easy_click` = `Alt`, so Alt+click never reached Ferrite. That blocked row 20. Alt+Space opens the xfwm4 window menu, so the command palette (default Alt+Space) was never reached. Alt+F4 worked. The WM setting was not changed.
- **Synthetic input.** Keys and clicks were injected as synthetic X events (xdotool-style) by an automated tester, not typed on a real keyboard. Fast injected key sequences may be dropped or reordered differently from real typing. This may have contributed to:
  - ISS-7: the auto-close backtick behaviour while typing a fence character by character. It reproduced 3 times, and the log shows Ferrite's own auto-close inserting a backtick after each keystroke.
  - ISS-18: the first keystroke dropped in Settings search or shortcut capture.
  - ISS-12: Ctrl+, not opening Settings. (Update, Batch 12b: Ctrl+, is received and logged, so this part is a real defect; see ISS-37.)
  Please re-check these three with a real keyboard before acting on them.
  Batches 5–6 add two input gaps that are recorded as BLOCKED or PARTIAL, not FAIL: the mouse drag-select in row 34, and Alt+Z in row 39 (the synthetic Alt+Z arrived as a plain `z`). Ctrl+, again did not open Settings (SI-4).
- **Shared config directory.** `~/.config/ferrite/` held state across launches: session, recovery files, config. Hangs had to be cleared with `kill -9`, so several later launches opened with a Session Recovery dialog ("Start Fresh" was usually chosen). Before the first GUI launch, `~/.config/ferrite` already existed because `cargo test` had written to it (ISS-28). It was moved aside to `~/.config/ferrite.bak-1791483152`.
- **Network.** Network access was available; logs show `img.youtube.com` reached through `198.18.0.1`. The offline test (EXTRA-YT-2) used `sudo unshare -n`.
- **Hang diagnostics.** Hangs were diagnosed with `gdb -p <pid> -batch "thread apply all bt 40"` (`hang-dump.sh`) against the release binary. Function names resolve, but inlined frames are elided.
- **Screenshots.** Some rows have only in-session screenshots that were not saved as files: T1-19, EXTRA-SET-1, -4, -8, -11, and part of T2-24. They are listed as "none" in Evidence.
- **Batches 7–12 input limits.** The synthetic input dropped modifiers at times (Shift in Shift+F3, Alt in Ctrl+Alt+Shift chords, Ctrl in Ctrl+Shift+F), dropped typed characters (queries were pasted from the clipboard instead), and often did not register mouse drags (CPY-2/CPY-1 selections, IMG-2 drag-and-drop, title-bar drags, Mermaid hand-pan). Clicks sometimes went through panels to the tab strip. The native file dialog fails on this box (xdg-desktop-portal), so files were opened through single-instance forwarding. Rows affected only by these limits are BLOCKED or PARTIAL, not FAIL. ISS-62 (Alt+Z) and the EXTRA-SET-6 capture behaviour need a real-keyboard check.

## Automated gate
| Check | Result | Numbers |
|---|---|---|
| cargo build --release | PASS | warnings: 26 (`ferrite` bin), 0 errors, plus 1 future-incompatibility notice: "packages contain code that will be rejected by a future version of Rust: ashpd v0.8.1". 273 s. Evidence: `logs/build-release.log`, `logs/gate-timing.log` |
| cargo test --locked   | PASS | 2067 passed / 0 failed / 2 ignored (2069 tests, 346 s wall incl. compile, suite 0.92 s). The Windows reference is 2078 passed, so 11 fewer tests ran on Linux. Most likely these are platform-gated tests, but that was not verified. Evidence: `logs/cargo-test.log` |
| cargo clippy (`--all-targets`) | PASS (0 errors) | **0 errors**. Warnings: 300 for bin `ferrite`, plus 375 for bin `ferrite` (test) of which 287 are duplicates, so **≈388 unique**. 113 s. Evidence: `logs/cargo-clippy.log` |

Gate notes:
- **Clippy warnings worth looking at.**
  - `clippy::suspicious_open_options` ("file opened with `create`, but `truncate` behavior not defined") at **`src/single_instance.rs:90`** and **`src/single_instance.rs:137`**. *Correction to the hand-off notes:* they cited `:137` and `:384`. The clippy log shows `:384` is a `collapsible_if` warning in the same file, and the second open-options site is `:90`.
  - Deprecated **`egui::Context::run`** ("Call run_ui instead") at `src/markdown/widgets.rs:6242`, plus "unused return value of `egui::Context::run` that must be used" at the same line.
  - Deprecated `egui::CentralPanel::show` at `src/markdown/widgets.rs:6243`.
  - Unused imports `Cell`, `RefCell` at `src/markdown/video_webview_input.rs:15`.
  - Biggest categories: field assignment outside initializer with `Default::default()` (39), unnecessary `to_string` (32), needless borrow (25), manual prefix stripping (21), too many arguments (20+8+4+3).
- **`cargo test` writes to the real user config dir** (`~/.config/ferrite/config.json` and `recovery/`), per the prep observation in `gui-test-plan.md` §1. The directory existed before the first GUI launch and was moved aside in Batch 0 step 2. See ISS-28.

## Results — minimal release test
Log names are abbreviated: `…-HHMMSS-tag.log` = `logs/ferrite-debug-20261008-HHMMSS-tag.log`. Severity for Tier 1 FAILs is **blocker**, per brief §8 ("If any Tier 1 row fails … mark it blocker").

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| 1 | PASS | shots/T1-1-pass.png; `…-201237-session1.log` | Rendered → click "First paragraph." → Esc: tab not dirty. Disk md5 unchanged, CRLF 8/8. |
| 2 | **FAIL (hang)** — blocker | shots/T1-2-fail.png, shots/T1-2-blocked-hang.png, shots/T1-2-blocked-hang-retry.png; logs/T1-2-hang-bt.txt; `…-203933-HANG-A.log` (+ `…-201237-session1.log`, `…-201751-T1-2-retry.log`) | 3 attempts, all frozen (exit 137 after kill -9). Typed text is inserted ("First pa editedragraph."). Clicking elsewhere to leave the block freezes the UI (ISS-1). Ctrl+S and the EOL check were never reached. The disk file is unchanged (CRLF 8/8). The first two attempts were logged as BLOCKED; the third, with a gdb backtrace, is the FAIL record. |
| 3 | PASS | logs/T1-3-check.txt; `…-201938-T1-3.log` | `RESULT: CRLF preserved (True)` 8/8. `cat -A`: every line ends `^M$`. |
| 4 | PASS | logs/T1-4-check.txt; `…-201938-T1-3.log` | Alt+↑/↓, Ctrl+Shift+D, Ctrl+Shift+U (status "TOC inserted (2 headings)") → CRLF True 16/16, TOC lines end `0d0a`. `file` says "exported SGML document" because of `<!-- TOC -->`, which is not an EOL problem. |
| 4b | **FAIL** — blocker (must-pass) | shots/T1-4b-fail.png; logs/T1-4b-check.txt; `…-202232-T1-4b.log` | ISS-3. The UTF-16 LE file never opens in Ferrite ("Delegating to external app (binary)"); LibreOffice opened it instead. The fixture itself is valid (BOM `fffe`, 8 `0d00 0a00` pairs, no lone `0a00`), so the hex condition only "holds" because Ferrite never touched the file. |
| 5 | PASS | `…-202232-T1-4b.log` | External `sed` + Ctrl+Shift+R → content updated, no dialog, status "Reloaded: reload-test.md". |
| 6 | PASS | shots/T1-6-pass.png; `…-202232-T1-4b.log` | Dialog appeared. Cancel kept the edit; Don't Save loaded the disk content including "external edit". |
| 7 | **FAIL (hang)** — blocker | shots/T1-7-pass.png (dialog), shots/T1-7-fail-hang-after-dialog.png, shots/T1-7-fail-hang.png; logs/T1-7-hang-bt.txt; `…-204439-HANG-B.log` (+ `…-202232-T1-4b.log`) | The row's own expectation was met twice: the Unsaved Changes dialog appears for an uncommitted Rendered edit. Clicking **Cancel** then froze the app both times (ISS-1, kill -9). Counted as FAIL because the app becomes unusable (brief: a crash is always FAIL). |
| 8 | PASS | `…-202755-T1-8.log`; shots/T1-8-dialog-open.png (reference) | With the reload dialog open, closing another tab and then Don't Save reloaded the right tab ("ext2"). other-tab.md md5 unchanged. |
| 9 | PASS | shots/T1-9-exit-dialog.png (reference); `…-202755-T1-8.log` | Alt+F4 → Save → both files updated, exit 0, no dialog loop. |
| 10 | PASS | `…-203037-T1-10.log`, `…-203128-T1-10b.log` | Don't Save → relaunch: no edit, no banner, no dialog. Observation: WARN "Ignoring stale recovery file for tab 1" pruned a leftover `recovery/1.json`. |
| 11 | PASS | shots/T1-11-pass.png, shots/T1-11-observation-no-dialog-after-undo.png; `…-203215-T1-11.log`, `…-203306-T1-11-relaunch.log`, `…-203405-T1-11-shot.log`, `…-203534-T1-11-dialog.log` | Restore gives the edit back, and one Ctrl+Z returns to the disk content. Observations filed as ISS-20 (raw i18n key), ISS-22 (banner persists), ISS-23 (stale recovery file). |
| 12 | PASS | `…-204753-batch2.log` | `§`, `é`, `中` at line start, no crash. |
| 13 | PASS | `…-210253-batch2.log` | Ctrl+Shift+D / Alt+↑ / Alt+↓ / Ctrl+D on `日本語テキスト ééé`: no crash, caret sane. The Alt+↑ swap of two identical lines can't be seen. The status-bar line count was stale (ISS-25). |
| 14 | **FAIL** — blocker | shots/T1-14-fail.png; `…-204753-batch2.log` | ISS-5. Ctrl+V with a PNG on the clipboard inserts nothing (`arboard paste error … incorrect type received from clipboard`). Right-click › Paste did insert `![](assets/1791485625-clipboard.png)` and wrote `fixtures/assets/1791485625-clipboard.png`. |
| 15 | **FAIL** — blocker (must-pass) | shots/T1-15-fail.png; `…-204753-batch2.log` | ISS-4. **No crash** (the old #178 crash is gone). 60 matches in 26 files, file listed. Clicking the result puts the caret at **Ln 1, Col 1**, not on `中文` (Col 6). Reproduced twice. |
| 16 | PASS (must-pass) | shots/T1-16-pass.png; `…-204753-batch2.log` | Whole word `中文` → 1 of 1. Whole word + Aa `é` → only the standalone `é`. No crash. The first frame said "No matches" until the pointer moved (repaint lag, ISS-24). |
| 16b | PASS | `…-204753-batch2.log` | `日本語日本語 [hello](https://example.com) world`. Log: "Smart paste: Created link … at char 69..74". |
| 17 | **FAIL (hang)** — blocker | shots/T1-17-fail.png, shots/T1-17-retry-fail-hang.png; logs/T1-17-hang-bt.txt, logs/T1-17-retry-hang-bt.txt; `…-204753-batch2.log`, `…-210253-batch2.log` | ISS-1. On the retry, **one** ArrowDown after clicking the first heading in Rendered froze the app. Paragraph integrity could not be checked. |
| 18 | **FAIL (hang)** — blocker | shots/T1-18-fail-hang.png; logs/T1-18-hang-bt.txt; `…-210656-batch2c.log` | ISS-1. ArrowDown #1 fine; ArrowDown #2 toward the blockquote froze. `logs/T1-18-not-hung-bt-taken-by-mistake.txt` is a dump of the same process taken *before* the freeze (main thread idle in the event loop), kept for reference. |
| 18b | **FAIL (hang)** — blocker | shots/T1-18b-fail-hang.png; logs/T1-18b-hang-bt.txt; `…-210920-batch2d.log` | ISS-1. Edited the callout body ("callout bodyx"), clicked away, and the app froze. Prefix preservation (`> ## Titlex` etc.) could not be checked. Disk unchanged. |
| 19 | **FAIL** — blocker | no screenshot saved; `…-211300-batch2e.log` | ISS-6. One Ctrl+Z after a preview code-block edit removed only the last character ("Undo (14 remaining)"). SENTINEL-LINE-1/2 stayed intact across the Ctrl+Y/Ctrl+Z cycles. *Note:* results-batch2 names the log as `…-210920-batch2d.log (launcher tag batch2e)`, but the Split-undo events are in `…-211300-batch2e.log`. |
| 20 | BLOCKED (environment) | shots/T1-20-fail.png (state after the supplementary Ctrl+click + Tab), shots/T1-20-probe.png, shots/T1-20-probe2.png; `…-212111-T1-20.log` | xfwm4 swallows Alt+click (`easy_click = Alt`): 2 attempts, no extra caret. **Supplementary, not the row's gesture:** with lines 7–9 selected, Ctrl+click on line 33 added a caret. Tab indented lines 7–9 by 4 spaces and inserted 4 spaces at the extra caret. One Ctrl+Z restored the document exactly (1465 → 1449 chars). The source adds extra carets on **Ctrl**+click (`src/editor/ferrite/editor.rs` ~L2501), so the checklist's "Alt+click" may be stale. |
| 21 | **FAIL** — blocker (must-pass) | shots/T1-21-fail.png, shots/T1-21-malformed.png, shots/T1-21-supp-clean-paste-diagram.png; `…-212111-T1-20.log` | ISS-7. **No crash** (exit 0): the malformed lines `A}x{ --> B` and `subgraph a] [b` don't crash, so the old PRD §5.2 crash is fixed. But typing the fence character by character produces a corrupted fence (a run of backticks, then `diamrem`). Reproduced 3 times. A pasted clean block, including the `subgraph 业务 PEP` line, renders a flowchart. |
| 22 | **FAIL** — blocker | shots/T1-22-fail.png, shots/T1-22-fail-hang.png, shots/T1-22-probe.png, shots/T1-22-hover-probe.png; logs/T1-22-hang-bt.txt; `…-213134-T1-22.log` (hang), `…-213643-T1-22b.log` | ISS-8 + ISS-1. Fit-to-pane OK (no horizontal scrollbar), no gap below the diagram, no overpaint, wheel scroll over the diagram OK. **Popup never opens** (6 clicks plus Ctrl+click). **No Fit/Native toggle**, and no hover tooltip or hand cursor. Session 1 **hung** when the expand icon was clicked while a paragraph was in edit mode. |
| 23 | PASS | shots/T2-23-pass-zoom.png, shots/T2-23-pass-bold.png, shots/T2-23-pass.png; `…-213643-T1-22b.log` | `zoom` → Font Size (+ Keyboard Zoom rows); `bold` → Format › Bold Ctrl+B; Esc clears. Glitches: the first "bo" was dropped once (ISS-18); Keyboard chip shows (1) for 3 rows (ISS-15). |
| 24 | PASS | shots/T2-24-pass.png; `…-213643-T1-22b.log` | Each chip shows its section, and `tab` shows live counts. The Keyboard count is wrong (ISS-15). With a chip active plus a query, the chip filter is kept (Terminal (0) → "No settings match"). |
| 25 | PASS (deviation) | shots/T2-25-pass-before-quit.png, shots/T2-25-pass.png; `…-213643-T1-22b.log`, `…-214507-T2-25-relaunch.log` | Recently Changed listed "Font Size 18px" and "Theme: Dark", and they persisted after restart. The **keyboard rebind (Toggle Word Wrap → Ctrl+Alt+K) was not listed**, although the checklist expects it (code excludes rebinds by design). Max observed 2 entries, so the cap (checklist 6 vs code 8) was not exercised. ISS-16. |
| 25b | PASS (must-pass) | shots/T2-25b-pass.png; `…-214507-T2-25-relaunch.log` | Slider 14→20 in one drag, ending exactly at 20px (mid-drag values are not visible in a still). Accent picker stayed open across 3 picks. "qa-t2": all 5 chars landed. Side finding: the All-view scroll stops at Files › Auto-save delay (ISS-14). |
| 26 | PASS | shots/T2-26-pass.png; `…-214507-T2-25-relaunch.log` | "Ferrite v0.3.1". Check for Updates → "You're on the latest version!" Releases / Source / Issues links opened in the browser (issue list only; nothing created). |
| 27 | PASS | shots/T2-27-pass.png; `…-214507-T2-25-relaunch.log` | Ctrl+S on Reload → banner 'Conflicts with "Save"' and the binding is not applied. Ctrl+Alt+R applied, and reset restored Ctrl+Shift+R. The MD cheat-sheet Ctrl+B chip deep-links to Settings with "Bold" filled in. The first key press in chord capture was swallowed (ISS-18). |
| 28 | PASS (no confirm) | shots/T2-28-find-open.png, shots/T2-28-pass.png, shots/T2-28-pass-appearance.png; `…-214507-T2-25-relaunch.log` | Ctrl+F on a document opens document Find. **Reset All reverts immediately with no confirm dialog** (ISS-17). Defaults are restored and nothing crashes. |
| 29 | PARTIAL | shots/T2-29-*.png (9 files incl. T2-29-pass-shell-search.png); `…-214507-T2-25-relaunch.log`, `…-215656-set-extra.log` | No raw locale keys anywhere. Every listed control was found **except a terminal-shell selector** (ISS-13; the expectation may be stale because the registry has no shell entry). Odd: Open Workspace and Close Workspace are both bound to F12 (ISS-27). |
| 30 | PASS | shots/T2-30-rendered-enter1.png, shots/T2-30-rendered-match7.png, shots/T2-30-rendered-shift-f3-6of7.png, shots/T2-30-split-enter1.png, shots/T2-30-split-f3-wrap.png, shots/T2-30-split-shift-f3-wrap.png; `…-225519-T2-30.log` | Rendered: Ctrl+F `ferrite` → 7 matches. Enter steps 1→7 and the viewport scrolls to each match (log scroll offsets 0 … 1143 px). F3 at 7 wraps to 1 and scrolls back to the top; Shift+F3 wraps 1→7, then 7→6. Ctrl+F again pre-selects the query, and typing replaces it. Split: the raw pane highlights and scrolls, the preview scrolls to the same matches; wrap and the Ctrl+F re-select also work there. Exit 0. Right after typing, one frame showed a stale "No matches" until the next repaint (ISS-24); the log has 7 matches. |
| 31 | PASS | shots/T2-31-rendered.png; `…-225932-T2-31.log` | Rendered: `&amp; &lt; &gt; &quot; &nbsp; &rarr; &larr; &reg; &copy; &mdash; &ndash; &hellip;` → `& < > "` (nbsp as a space) `→ ← ® © — – …`. `&amp; &rarr; &reg;` → `& → ®`; numeric `&#38; &#8594; &#174;` → `& → ®`. `&foo;` stays literal. `<span>tag</span>` is not executed (grey `«HTML»` chips around plain `tag`). Exit 0. |
| 32 | **FAIL** — major | shots/T2-32-raw.png, shots/T2-32-rendered.png, shots/T2-32-rendered-recheck.png; `…-230020-T2-32.log` | ISS-30. Noto Color Emoji is installed and the log registers it as the emoji fallback, but every emoji is drawn monochrome, and 2 of the 7 common emoji on line 9 (`😀 😅 🤔 😴 🥳 😢 😡`) are blank: no glyph, no tofu (Raw: the last two; Rendered: the 4th and 7th, same after about 1 min). VS16 after ⚠ shows an empty box, ZWJ sequences split into components (one empty box), flags show as letters `N O U S J P D E`. Toolbar and tab icons are unchanged. No crash, exit 0. |
| 33 | PASS | shots/T2-33-rendered-top.png, shots/T2-33-rendered-scrolled1.png, shots/T2-33-rendered-scrolled2.png; `…-230432-T2-33.log` | `assets/`, `./assets/`, bare name and backslash variants all render the sample image (percent-encoded `sample%2Epng` too). The remote `https://example.com/…png` shows a "Web images not supported" placeholder; 0 log lines match ureq/http/example.com, so nothing was fetched. A missing file shows an "Image not found" placeholder with one WARN, no retry storm. Smooth scrolling, exit 0. |
| 34 | BLOCKED (drag-select) | shots/T2-34-rendered-top.png, shots/T2-34-rendered-selected5.png, shots/T2-34-rendered-selected6.png, shots/T2-34-paste-after-copy.png; `…-230533-T2-34.log`, `…-231936-T2-34-paste.log` | Six synthetic mouse drags never selected the whole bash block (nothing, partial ranges, or the selection cleared). Ctrl+C on a partial selection left the clipboard unchanged and logged no copy event. **Supplementary, not the row's path:** the block's own Copy button put exactly 4 lines on the clipboard, and `xclip -selection clipboard -o` was byte-identical to source lines 12–15 (173 bytes, `mkdir -p` intact, no inserted or trailing spaces). The Rust block and the inline paragraph were not copied. After quit the clipboard was empty (no clipboard manager on the box; arboard WARN). No crash, exit 0. |
| 35 | **FAIL (hang)** — blocker (= ISS-1) | shots/T2-35-split-raw-typed-Z.png, shots/T2-35-rendered-lock-typed-Q.png, shots/T2-35-rendered-lock-on.png, shots/T2-35-fail.png, shots/T2-35-clean-relaunch-rendered.png, shots/T2-35-clean-relaunch-toggled-off.png; logs/T2-35-hang-bt.txt, logs/T2-35-relaunch-hang-bt.txt; `…-230821-T2-35.log`, `…-231259-T2-35-relaunch-a.log` (both exit 137), `…-231800-T2-35-clean-relaunch.log` | ISS-31, the same deadlock as ISS-1. With the lock ON, the Raw pane in Split stays editable (typed Z, Ctrl+Z restored it), and typing Q in Rendered changes nothing: PASS. Arrow keys in Rendered: 3 Down presses did nothing visible; after a click on a blank area of the pane, 1 Down froze the app (`arrow_nav_sel_key` ← `remember_arrow_nav_sel` ← `show_rendered_editor`). It froze in 2 launches, kill -9 both times. Lock persistence: PASS after the 2nd kill and after a clean quit + relaunch (after the 1st kill the padlock was open and was turned ON again). Fixture unchanged. |
| 36 | **FAIL (crash)** — blocker | shots/T2-36-fail.png; `…-223058-T2-36.log`, `…-223148-T2-36-retry.log`, `…-223237-T2-36-split.log` | ISS-2. Switching the fixture to Rendered (2×) or Split (1×) aborts with exit 134: `GDK has not been initialized`. Ctrl+P, resize, save, and the source rewrite were unreachable. |
| 37 | PASS (play-video sub-step BLOCKED by ISS-2) | shots/T2-37-forward-window2.png, shots/T2-37-pass.png; `…-232204-T2-37.log`, `…-232639-T2-37-relaunch.log` | Ctrl+Shift+N opened a 2nd OS window in the same process (the first press did not register). With window 2 focused, `$FER /tmp/lf-test.md` from a second shell returned in 0.088 s; the file landed as a new tab in window 2 (last-focused), which became active, and window 1 was unchanged. Window 2 closed with its X at once, no prompt. Quit with 2 windows via Alt+F4 on the root window (there is no Ctrl+Q or File › Quit), exit 0. The relaunch restored both windows with their tabs in saved order ("Restored 11 of 11 tabs from session (2 windows)"). Play-video sub-step not attempted (YouTube crash). The restore pre-scan logged "Restoring 14 tab(s)" (see observations). |
| 38 | PASS (must-pass) | shots/T2-38-pass.png, shots/T2-38-relative-pass.png; `…-232639-T2-37-relaunch.log` | `$FER /tmp/lf-test.md` from a second shell: 0.100 s, rc 0, no extra process left. The file opened as a new active tab, and the window was raised and became active (it was the only Ferrite window). `cd /tmp && $FER ./lf-test.md`: 0.066 s, rc 0, and the **relative path opened** (resolved to /tmp/lf-test.md), no "file not found". Forwarding processes write no log by design. |
| 39 | PARTIAL (Alt+Z sub-step BLOCKED) | shots/T2-39-contextmenu.png, shots/T2-39-newtab-pass.png, shots/T2-39-close-pass.png, shots/T2-39-copypath-after.png, shots/T2-39-reveal-pass.png, shots/T2-39-reorder-pass.png, shots/T2-39-altz-before.png, shots/T2-39-altz-attempt4.png; `…-232639-T2-37-relaunch.log` | Tab context menu: New Tab, Close Tab (no prompt), Copy Path (`xclip` returned the exact absolute path) and Reveal in Explorer (thunar opened `test_md/`) all PASS. Drag-reorder PASS on the 3rd drag ("Reordered tabs: 2 <-> 1"). **Alt+Z unverified:** in 3 attempts the synthetic Alt+Z arrived as a plain `z` typed into the buffer (removed, never saved); a 4th, with focus on the tab strip, did nothing. No word-wrap toggle in the log. ToggleWordWrap is bound to Alt+Z in `src/config/settings.rs:906`, so this needs a real-keyboard check. (`results-batch5.md` records the row as BLOCKED for the Alt+Z sub-step.) |
| 40 | SKIPPED (platform) | — | Windows title-bar row (re-confirmed in Batch 5b). The Linux equivalent, TTL-6, was not reached. |
| 41 | PARTIAL | shots/T3-41-partial-rendered-alignment.png, shots/T3-41-partial-raw-after-edit.png, shots/T3-41-supp-alignment-center-right-table_wrapping.png, shots/T3-41-supp-inline-format-task50.png; `…-233605-T3-41.log` (edit), `…-233832-T3-41.log`, `…-233908-T3-41.log` (supplementary) | The named fixture `test_tables.md` has no `:-:` or `--:` column (its "Table with Alignment" separator is left/left/left) and no formatted cells, so alignment and inline formatting can't be checked on it. Cell edit PASS: typing Z into a Rendered cell gave "TextZ" with the layout intact, and Raw line 42 changed with the separator row intact (the first switch to Raw showed stale text until a Rendered round trip). Discarded with Don't Save. Supplementary fixtures: `test_table_wrapping.md` renders `:-:` centred and `--:` flush right; `task_50_table_inline_formatting.md` renders bold, italic, strikethrough, code and links in cells, **but `[**bold link**](https://example.com)` shows literal `**`** (ISS-35). |
| 42 | PASS | shots/T3-42-pass.png; `…-233959-T3-42.log` | `test_data.csv` grid: double-click "New York", End, type X → "New YorkX", tab `*`. Tab moved focus to the next cell. Ctrl+Z reverted to "New York" and cleared `*` ("Undo (0 remaining)"). Not saved. |
| 43 | **FAIL** — major | shots/T3-43-fail-subgraph-test-td-title-overlap.png, shots/T3-43-fail-edge-routing-td-title-overlap.png, shots/T3-43-fail-nested-direction-override-overlap.png, shots/T3-43-fail-dash-edge-label-overlap.png, shots/T3-43-gitgraph-gg01-02.png, shots/T3-43-gitgraph-gg03.png; `…-234101-T3-43.log` | ISS-32. Every Mermaid block renders, with no error box or crash. `test_flowcharts.md` has overlaps in 4 diagrams: edges run through subgraph titles, a nested subgraph covers its parent's title, and an edge label is clipped behind a node, while one label, node "Flag" and its edge are not drawn. 3 more nested-subgraph diagrams don't match their Expected text. `test_git_graphs.md` GG-01…03: no node or label overlap, but lane and commit labels are ellipsised. GG-02's feature lane forks from HF1, which matches the source; the fixture's Expected text says BASE. |
| 44 | **FAIL** — major | shots/T3-44-fail-adjacent-single-line-divs.png, shots/T3-44-pass-kbd.png, shots/T3-44-adjacent-divs-check.png; `…-234543-T3-44.log` | ISS-33. Multi-line `<div align="left/center/right">` blocks render correctly. `<details>` is closed by default, `<details open>` is open, and clicking the summary toggles it. `<kbd>` renders key caps. **Fail:** the two adjacent single-line `<div align>` blocks under "Adjacent divs" are not rendered; the document's first line is drawn twice in their place. `<strong>` inside a summary shows as grey tag chips, not bold. No crash. |
| 45 | PASS | shots/T3-45-pass-output-attached-after-edit.png; `…-234736-T3-45.log` | §2c Python block: Run → consent dialog → "Enable & run" → output "Exit 0 · 21ms", "Hello from Python". After typing ` EDITED` on Raw line 4 (above the fence), the output was still attached to the same block in Rendered. Not saved (Don't Save, then `git checkout`). |
| 46 | PASS | shots/T3-46-pass-outline-jump.png; `…-234912-T3-46.log` | New Untitled doc with `## 1\.3\.1 Title` on line 19. The Outline panel (Ctrl+Shift+O) shows "1.3.1 Title" without backslashes; clicking it moved the caret to Ln 19, Col 1. The Don't Save step could not be done: Alt+F4 on the modified Untitled doc gave **no** Unsaved Changes prompt, and the text went only into crash recovery (see observations). |
| 47 | PARTIAL | shots/T3-47-pass-pdf-viewer.png, shots/T3-47-pass-image-viewer.png; `…-235054-T3-47.log`, `…-235343-T3-47.log` | From the file tree, `sample.pdf` opens a PDF viewer tab and `icon-192x192.png` an image viewer tab: PASS. `sample.zip` and `FerriteMDPortable.exe` log "Opened in default application" + "Delegated file tree selection to external app"; no Ferrite tab, no crash. On this box xdg-open hands both to Chrome, which came to the front and **downloaded** them to `~/Downloads` (the exe was not run). The Ferrite toast was **not observed**, because Chrome covered Ferrite on every click. Both launches exit 0. |

## Results — total test (rows not already covered)

### Rows run directly (Batch 4: video + EOL-7; VID-10 and PLT-2 verdicts derived)
| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| VID-1 | **FAIL (crash)** | shots/VID-1-fail-crash.png; `…-223520-VID-1.log` | ISS-2. Minimal doc with one trusted `youtube.com/watch` embed: Rendered aborts (exit 134, 22:35:37). No thumbnail, no play overlay. (The total test had this pre-marked `[x]` from an earlier Windows pass.) |
| VID-2 | BLOCKED (crash) | shots/EXTRA-YT-6-fail-crash.png; `…-223957-EXTRA-YT-6.log` | Not run alone. Every trusted embed goes through the aborting path; a typed `youtu.be` embed in Split also aborted (EXTRA-YT-6). |
| VID-3 | BLOCKED (crash) | shots/T2-36-fail.png; `…-223058-T2-36.log` | `width=480` / `320×240` never rendered. |
| VID-4 | PASS | shots/VID-4-pass.png, shots/VID-4-cases.png; `…-223312-VID-4.log` | Vimeo shows the placeholder "External video (opens in browser)". 0 webview/create_child log lines, no crash, same in Split. Card not clicked. |
| VID-5 | PARTIAL | shots/VID-4-pass.png; `…-223312-VID-4.log`, `…-223549-CTRL-plain.log` | No embed created (correct). But bare URLs and `<https://…>` autolinks render as plain black text, not as links. A control doc with no video behaves the same, so this is not embed-specific (ISS-26). |
| VID-6 | BLOCKED (crash) | shots/T2-36-fail.png | The drag handle exists only on a rendered trusted embed. |
| VID-7 | BLOCKED (crash) | — | Needs a playing video. |
| VID-8 | BLOCKED (crash) | — | Needs a playing video. |
| VID-9 | BLOCKED (crash) | — | Needs a playing video in the primary window. |
| VID-§9 | BLOCKED (crash) | shots/EXTRA-YT-4-fail-crash.png; `…-223923-EXTRA-YT-4.log` | Same-video ×4 fixture and the 12-embed doc both abort on Rendered. |
| VID-§10 | BLOCKED (crash) | — | Needs a live player. |
| VID-§11 | BLOCKED (crash) | — | Needs a live player. |
| VID-10 | **FAIL (crash)** — = ISS-2 (verdict derived from VID-1, T2-36, EXTRA-YT-2/4/6/7; not a separate run) | shots/VID-1-fail-crash.png, shots/EXTRA-YT-7-fail.png; logs/build-release.log | First half holds: the release build compiles `webkit2gtk-sys v2.0.2` and `wry v0.55.1` and links WebKitGTK 4.1. Second half ("or degrades to thumbnail; no crash") fails: every trusted embed aborts with `GDK has not been initialized` (9 launches, exit 134). |
| PLT-2 | PARTIAL (derived) | logs/build-release.log | Release build with the WebView deps: PASS (gate; `webkit2gtk-sys`/`wry` compiled). The gui-test-plan's confirmation step ("PASS if VID-1 plays") is not met because VID-1 aborts (ISS-2). Report-writer check on Oct 9 (read-only): `ldd target/release/ferrite` lists `libwebkit2gtk-4.1.so.0`. |
| EOL-7 | PARTIAL | logs/EOL-7-check.txt; `…-223753-EOL-7.log` | Raw only, because Rendered would abort: x + Backspace + Ctrl+S → CRLF True 5/5, `0d0a`. The drag-resize part, which is the actual subject of the row, was **not** performed (needs Rendered). |

### Rows run in Batches 9–12 (Oct 9 2026)
Every row below is from `results-batch9.md` … `results-batch12.md`. Where a row ran more than once, the latest complete run is used. Debug-log times in the notes are UTC (`Z`).

#### Batch 9a — §1 status bar, line endings, session

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| STAT-3 | **FAIL** — minor | shots/STAT-3-fail.png; logs/ferrite-debug-20261009-023201-STAT-3.log | ISS-49. Untitled, Raw: `- item`, `* item`, `1. item`, each + Enter → caret on an empty line at Col 1, **no list marker continued**. Status-bar stats stay correct ("6 words, 21 chars, 3 lines" → "6 words, 22 chars, 4 lines"), no panic. `split_list_item` (src/markdown/ast_ops.rs) is only called from the Rendered editor. Reported from the sequential re-run (a first parallel attempt reordered the keys). Triage: the row text ("list detection / stats unchanged") may only require the stats. |
| EOL-3 | PASS | shots/EOL-3-pass.png; logs/ferrite-debug-20261009-023554-EOL-3.log, logs/EOL-3-check.txt | `/tmp/crlf-test.md`, Split, typed ` x` in the preview, Ctrl+S → CRLF preserved (True), 8/8 lines `^M$`. Observation: the preview edit did not reach the Raw pane, status bar or dirty marker until Ctrl+S (ISS-41). |
| EOL-5 | PASS | shots/EOL-5-pass.png; logs/ferrite-debug-20261009-023739-EOL-5.log, logs/EOL-5-check.txt | `/tmp/lf-test.md`, Rendered edit ` x`, Ctrl+S → LF-only (True), no `^M`. Same late status/dirty update as EOL-3. |
| EOL-9 | **FAIL** — minor (optional row) | shots/EOL-9-fail.png; logs/ferrite-debug-20261009-023851-EOL-9.log, logs/EOL-9-check-pre.txt, logs/EOL-9-check.txt | ISS-50. Mixed file (4 of 5 lines CRLF): edit ` x` at the end + Ctrl+S → still "with CRLF, LF line terminators"; `line lf$` kept its LF, each line kept its own ending. No crash. |
| SES-1 | **FAIL** — major | shots/SES-1-fail-dialog.png, shots/SES-1-fail.png; logs/ferrite-debug-20261009-024031-SES-1.log | ISS-38. Two dirty files + one dirty Untitled tab → Alt+F4 → one dialog "Unsaved Changes - You have unsaved changes. Exit anyway?" (Save / Don't Save / Cancel, no per-tab list). **Save**: both files saved (disk verified), then exit 0 with **no Save As for the Untitled tab**; its text went only to session recovery (`Saved recovery content for tab 69 (path=None)`). `fixtures/untitled-save.md` was not created. |
| SES-3 | PASS | shots/SES-3-pass-dialog.png, shots/SES-3-pass-after.png; logs/ferrite-debug-20261009-024315-SES-3.log | Ctrl+Shift+N (the first press was lost) → window 2. The native Open dialog fails on this box (rfd xdg_desktop_portal `pick_files error No such file or directory`), so a fresh `/tmp/ses3-w2.md` was forwarded into window 2 instead of `/tmp/ses1-a.md` (deviation). Edit → window 2's own close → "This window has unsaved changes. Close anyway?" → Save → `Saved file: /tmp/ses3-w2.md`, window 2 closed, window 1 stayed. |
| SES-4 | PASS | shots/SES-4-pass-tab-dialog.png, shots/SES-4-pass-window-dialog.png, shots/SES-4-pass-after.png; logs/ferrite-debug-20261009-024315-SES-3.log | (a) Tab × on dirty `ses1-b.md` → "'ses1-b.md*' has unsaved changes. Close anyway?" → Don't Save: disk unchanged, recovery 70 deleted. (b) Window 2 close → Don't Save: disk unchanged, recovery 75 deleted. (c) Recovery files of the discarded docs are gone; the remaining ones belong to still-open tabs (not mapped one by one). |
| SES-5 | PASS | shots/SES-5-pass-restored.png; logs/ferrite-debug-20261009-024315-SES-3.log, logs/ferrite-debug-20261009-024826-SES-5.log | Untitled `quick note SES-5` → Alt+F4 quit **with no prompt at all**; relaunch restored the Untitled tab with the text (still dirty). With only Untitled tabs dirty the app quits silently and relies on recovery (cf. SES-1, T3-46). |
| SES-6 | PARTIAL — ISS-63; possible data loss ISS-36 | shots/SES-6-window2-dialog.png, shots/SES-6-window1-dialog.png; logs/ferrite-debug-20261009-024942-SES-6.log | Window 2 close and app exit both show the right kind of dialog, but **neither names the file** (ISS-63); the single-tab dialog does. Don't Save in both → disk unchanged, exit 0. **Unexpected (ISS-36):** in window 1, click + End + typing ` W1` into the restored `ses1-a.md` left the editor showing only ` W1` ("1 words, 4 chars, 2 lines"), and recovery held only ` W1`: the restored text was gone from the buffer. Disk untouched because Don't Save was chosen. ` W2` in window 2 landed in the first paragraph, not at the end. |

#### Batch 9b — §2 entities, find, arrow navigation, copy, undo

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| ENT-6 | PASS | shots/ENT-6-pass.png; logs/ferrite-debug-20261009-025410-ENT-6.log | Split preview (view selector, no click into the preview): `&amp; &rarr; &reg;` → `& → ®`, `&#38; &#8594; &#174;` → `& → ®`, `&foo;` literal, `<span>tag</span>` not executed. The row's exact string is not in the fixture; the closest lines were checked. Same as T2-31. |
| FIND-5 | PASS | shots/FIND-5-pass.png; logs/ferrite-debug-20261009-025519-FIND-5.log | Raw, Ctrl+F `ferrite` → 7 matches. Enter/F3 forward 2…6 (viewport scrolls), Shift+F3 back 5…1, then wraps to 7 of 7 (viewport scrolls). xdotool dropped Shift on 4 of the Shift+F3 presses (logged as `F3 (Find Next)`), so the log was used to confirm each step. A brief "No matches" until the debounced search ran (ISS-24). |
| ARR-1 | **FAIL (hang)** — blocker (= ISS-1) | shots/ARR-1-fail.png, shots/ARR-1b-fail.png; logs/ARR-1-hang-bt.txt, logs/ARR-1b-hang-bt.txt; logs/ferrite-debug-20261009-025846-ARR-1.log, logs/ferrite-debug-20261009-030021-ARR-1b.log | Rendered `test_find_arrow_nav.md`: click into `The ferrite crystal sat on the shelf.` → **ArrowDown** → frozen (0% CPU, no repaint for 30+ s). Both attempts, same stack (`lock_shared_slow ← arrow_nav_sel_key ← remember_arrow_nav_sel`). kill -9 (exit 137). Up not tried (2-attempt cap). |
| ARR-2 | **FAIL (hang)** — blocker (= ISS-1) | shots/ARR-2-fail.png, shots/ARR-2b-fail.png; logs/ARR-2-hang-bt.txt, logs/ARR-2b-hang-bt.txt; logs/ferrite-debug-20261009-030131-ARR-2.log, logs/ferrite-debug-20261009-030412-ARR-2b.log | Section 4 multi-line paragraph (4 visual rows): click row 2, Down ×2 moves between visual rows correctly; the **3rd Down, which leaves the block, froze**. Twice, same stack. Pattern across ARR-1/2: a Down that changes block freezes, a Down within a block works. |
| ARR-4 | **FAIL (hang)** — blocker (= ISS-1) | shots/ARR-4-fail.png, shots/ARR-4b-fail.png; logs/ARR-4-hang-bt.txt, logs/ARR-4b-hang-bt.txt; logs/ferrite-debug-20261009-030543-ARR-4.log, logs/ferrite-debug-20261009-030712-ARR-4b.log | Split: Raw-pane arrows move only the raw caret and focus does not jump after a preview click (PASS part). Arrow keys with focus in the **preview** froze: Down (attempt 1), Up (attempt 2). Same stack, kill -9. |
| ARR-5 | PASS | shots/ARR-5-pass.png; logs/ferrite-debug-20261009-030834-ARR-5.log | Raw only: Down, Right, Left, Up each move the raw caret one step (status bar checked each time). No freeze. |
| ARR-6b | PASS | shots/ARR-6b-pass.png; logs/ferrite-debug-20261009-030935-ARR-6b.log | Second half of ARR-6 (first half = T1-17). New doc `## #1 issue` + `text`; in Rendered, End + ` x` in the heading → Raw shows `## #1 issue x`, still H2, not escaped to `\#1`. The status bar and dirty marker did not update while in Rendered (ISS-41). Not saved. |
| CPY-2 | **FAIL** — major | shots/CPY-2-fail.png; logs/ferrite-debug-20261009-031127-CPY-2.log, logs/CPY-2-selection-clipboard.txt | ISS-39. Rendered Part A, mouse selection + Ctrl+C on the rust line inserts spaces at token boundaries: `tems.iter().m` → `tems. iter(). m`; a longer selection → `let x: Vec< String> = items. iter(). map(\| s\| s.`. The block's **Copy** button (fallback) is byte-identical to fixture line 21, so the defect is in the selection path. Caveat: a clean full-line drag could not be made with the input tool; both partial selections show the spacing. |
| CPY-3 | **FAIL (hang)** — blocker (= ISS-1) | shots/CPY-3-fail.png, shots/CPY-3-blocked.png; logs/CPY-3-hang-bt.txt; logs/ferrite-debug-20261009-031538-CPY-3.log | About 4 ways to select the inline-formatted paragraph failed: drags scrolled the page; a double-click on `bold` put the paragraph into raw edit view and Esc or a margin click did not leave it. After a drag across that paragraph the app froze (xfwm4 "might be busy and is not responding"). Same `arrow_nav_sel_key` stack (caller `render_session_formatted_edit_text`). No clipboard text was produced, so nothing is claimed for the copy. |
| UND-3 | **FAIL** — major | shots/UND-3-fail.png; logs/ferrite-debug-20261009-031831-UND-3.log | ISS-40. Rendered (not Split), Part B Python block: (1) Edit, Ctrl+End, Enter, typing `UND3 added three words` kept only `U`, and the block left edit mode; (2) `alpha beta gamma` came out garbled (`return valuea b` / `eta gamma`). After Done, **each Ctrl+Z removes one character** (`Undo performed, 12 entries remaining`, then 11). SENTINEL lines intact; not saved. |

#### Batch 10 — §3 reload/encoding, indentation, emoji, images

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| RLD-3 | PASS | shots/RLD-3-pass.png; logs/ferrite-debug-20261009-032305-RLD-3.log | Dirty `/tmp/rld3.md` → Ctrl+Shift+R → "'rld3.md*' has unsaved changes. Reload from disk anyway?" (Save / Don't Save / Cancel) → Save → `Saved file: /tmp/rld3.md`, then "Reloaded: rld3.md", tab clean, edit on disk. |
| RLD-4 | PASS | shots/RLD-4-pass.png; logs/ferrite-debug-20261009-032305-RLD-3.log | Untitled tab → Ctrl+Shift+R → no dialog, no crash, toast "Nothing to reload — save the file first". |
| RLD-5 | PASS | shots/RLD-5-pass.png; logs/ferrite-debug-20261009-032305-RLD-3.log | Clean tab, external append, ribbon Save dropdown (Save / Save As… / Reload from Disk) → Reload from Disk → "Reloaded: rld3.md", new text shown. The menu click also leaked through to the tab strip (log `Switched to strip tab 11`); no edit resulted (counted as a tester-side artifact). |
| RLD-7 | PARTIAL — UTF-16 half = ISS-3 | shots/RLD-7-pass-cp1252.png, shots/RLD-7-fail-utf16.png; logs/ferrite-debug-20261009-032305-RLD-3.log, logs/RLD-7-cp1252-check.txt, logs/RLD-7-utf16-check.txt | **Windows-1252: PASS.** External append + Ctrl+Shift+R → `Café 2` readable; edit + Ctrl+S → `Saved file: /tmp/cp1252.md (encoding: windows-1252)`, `e9` kept, no `c3a9`. **UTF-16: FAIL.** `/tmp/crlf-utf16.md` is again `Delegating to external app (binary)` (LibreOffice opened it), so reload/edit/save can't be exercised in Ferrite; the hex check only re-reads the on-disk file. |
| IND-1 | PASS | none (state seen in in-session screenshots only); logs/ferrite-debug-20261009-032305-RLD-3.log | `/tmp/ind.md` (md5 = fixture), Raw. Lines 7–9 selected + Tab → all moved one step (4 spaces; 1449 → 1461 chars); Shift+Tab restored them (→ 1449); selection kept; Block 1 byte-identical after the round trip. |
| IND-2 | **FAIL** — minor | shots/IND-2-fail.png; logs/ferrite-debug-20261009-032305-RLD-3.log | ISS-51. Shift+Tab outdents space-indented lines one step but **not the hard-tab line**: selection 13–15 → 4→0 and 8→4 spaces, `\tindented with a hard tab` unchanged (disk-verified); caret-only on line 15 → no change. Caret-only on lines 13/14 → correct. Once (01:31:04, not reproduced later) a caret-only Shift+Tab on line 13 outdented line 14 instead. Ctrl+Shift+R sometimes did not register; reloads used the ribbon menu. |
| IND-3 | PASS | none (disk check); logs/ferrite-debug-20261009-032305-RLD-3.log | Use Spaces ON, Tab Size 4: Tab at column 1 of line 8, Ctrl+S → `    beta line two$`, exactly 4 spaces, no tab. |
| IND-4 | PASS | none (disk check); logs/ferrite-debug-20261009-032305-RLD-3.log | Use Spaces OFF: Tab at column 1 of line 9, Ctrl+S → `^Igamma line three$` (real tab). Setting restored to ON. |
| IND-5 | PASS | none (disk check); logs/ferrite-debug-20261009-032305-RLD-3.log | Caret mid-line (no selection), Tab → 4 spaces at the caret (Col 15 → 19); focus stays in the editor (a following Ctrl+V landed in the text): `alpha line one    Z$`. |
| IND-7 | PASS (anomaly flagged) | none (disk check); logs/ferrite-debug-20261009-032305-RLD-3.log | 4 carets via Ctrl+click (no add-cursor-below key exists), one Tab, Ctrl+S → each caret got 4 spaces (1451 → 1467). Ctrl+click past the end of a line adds no caret. **Anomaly, cause not established:** later, on the unsaved buffer that still carried the multi-cursor state, click + Shift+Down ×3 + Shift+End + Tab removed the text of lines 7–9 (1467 → 1424 chars); Ctrl+Z restored it exactly (sha256 equal); not reproduced after Esc. Earlier, keys meant for Settings search (`cu`) landed in the buffer at the carets (discarded). |
| IND-8 | PASS | none (disk check); logs/ferrite-debug-20261009-032305-RLD-3.log | Lines 31–34 (`§ section start`, `ß sharp s`, `中文行首`, `émigré`) + Tab → +4 spaces each, multibyte intact (1467 → 1483); Shift+Tab → identical to the pre-test copy (`diff`). The first Ctrl+S did not write; a second one did. |
| IND-9 | **FAIL (hang)** — blocker (= ISS-1) | shots/IND-9-fail.png; logs/IND-9-hang-bt.txt, logs/IND-9-tail.txt; logs/ferrite-debug-20261009-032305-RLD-3.log | Split (Ctrl+E gave Raw + Rendered): clicks on `list item one` / `nested item` in the Rendered pane + Tab/End showed no caret and changed nothing, then Ferrite froze (log silent from 01:40:44Z). gdb: `lock_shared_slow ← arrow_nav_sel_key ← remember_arrow_nav_sel ← render_session_plain_text_block ← render_list`. pkill -9. |
| EMO-4 | PASS | shots/EMO-4-pass.png; logs/ferrite-debug-20261009-034249-EMO-4.log | `test_emoji.md`, Raw only (Rendered avoided because of ISS-1): single-codepoint emoji, symbols and skin-tone lines draw glyphs; ZWJ sequences degrade to component glyphs (family → 3 persons, etc.). No crash, no blank cells *in this run* (contrast T2-32 / ISS-30, where 2 of 7 common emoji were blank). |
| EMO-5 | PASS | shots/EMO-5-pass.png; logs/ferrite-debug-20261009-034249-EMO-4.log | `test_chinese.md` (Raw) and the CJK line of `test_emoji.md` (`中文テスト한국어`) render Chinese, kana and hangul with no tofu. |
| IMG-2 | SKIPPED (no drag source) | shots/IMG-2-skip-dnd.png; logs/ferrite-debug-20261009-034249-EMO-4.log | `/tmp/img2/doc.md` opened; Thunar on `test_md/assets`. Three synthetic left-drags from `sample.png` only selected it; no drag-and-drop session started, nothing reached Ferrite, no `assets/` created. xdotool was not used as a lower-level substitute. |
| IMG-4 | BLOCKED (ISS-5) | none (buffer state in shots/IMG-7-lockstate.png); logs/ferrite-debug-20261009-034249-EMO-4.log | Ctrl+V with a PNG on the clipboard inserted nothing (`arboard paste error … incorrect type received from clipboard`, 01:44:58Z), so there was nothing for Ctrl+Z to remove. One attempt. |
| IMG-5 | PASS | shots/IMG-5-pass.png; logs/ferrite-debug-20261009-034249-EMO-4.log | Plain text → pasted as text. `https://example.com/a.png` → smart paste `![](https://example.com/a.png)` (log "Smart paste: Created image ![](url) …"), i.e. image-URL smart paste unchanged. No crash. |
| IMG-7 | BLOCKED (ISS-5) | shots/IMG-7-split.png, shots/IMG-7-lockstate.png, shots/IMG-7-lock-off.png; logs/ferrite-debug-20261009-034249-EMO-4.log | Preview lock ON/OFF toggles (padlock verified both ways). With the lock ON, Ctrl+V of the PNG in Raw inserted nothing (same arboard error, 01:45:38Z), so the gating of preview mutations could not be exercised. Lock turned OFF afterwards. |

#### Batch 11 — §4 Mermaid

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| MMD-1 | PARTIAL — ISS-8 (body click) | shots/MMD-1-partial.png, shots/MMD-1-hover-button.png; logs/ferrite-debug-20261009-035014-MMD-1.log | `test_mermaid_v031.md`, Rendered. Hover shows the expand icon (tooltip "Open diagram in popup"); **clicking it opens the popup** (Hand / Select / zoom % / + / − / reset / X). Zoom 100→119 %, 100→98 %, reset → 100 %; wheel in Hand mode zooms (229 %), not in Select mode; X closes; colours consistent. **Plain click and Ctrl+click on the diagram body: no popup** (4 tries; also 1 try on FC-83b, whose hover hint says "Click diagram or Ctrl+Click to open popup"). **Hand-pan BLOCKED**: 7 synthetic drags (2- to 20-point paths) never panned although the grab cursor showed. Select-mode drag not run. |
| MMD-2 | PASS | shots/MMD-2-pass.png; logs/ferrite-debug-20261009-035014-MMD-1.log | FC-83b: node F shows `Car` for `fa:fa-car Car`, no literal `fa:` text, no icon glyph. `test_mermaid_v031.md` §2: `Loading`, `Repo`, `Done` with no prefix text. |
| MMD-3 | **FAIL** — minor | shots/MMD-3-fail.png, shots/MMD-3-control-AB-crop.png, shots/MMD-3-controlC.png; logs/ferrite-debug-20261009-035014-MMD-1.log | ISS-57. §3 `linkStyle 0,1,2 interpolate basis` (collinear LR chain) draws straight segments. Scratch controls in `/tmp/qa11` (not in the repo): the same TD graph with and without `linkStyle 0,1,2,3,4 interpolate basis` renders pixel-identical (square elbows); `linkStyle default interpolate basis` draws smooth curves. So the indexed form has no effect. |
| MMD-4 | PASS (side note → ISS-32) | shots/MMD-4-pass.png, shots/MMD-4-popup-reset.png, shots/MMD-4-popup-z2.png; logs/ferrite-debug-20261009-035014-MMD-1.log | §4 wide subgraph title: S1 box border 399 px in every inline frame (3 static, 12 while scrolling) and 489 / 537 / 586 / 489 px in the popup at 100 % / zoom / zoom / reset: no flicker, title inside the box. Side note (not scored): subgraph `Short` overlaps the S1 box and its title is not drawn; S1/S2 render as plain nodes inside S1. |
| MMD-6 | **FAIL** — minor | shots/MMD-6-fail.png, shots/MMD-6-control.png; logs/ferrite-debug-20261009-035014-MMD-1.log | ISS-58. Valid hints applied (Start top-left, End lower-right, Middle auto; the A→B edge runs along Start's bottom border, cosmetic). **Invalid hint:** amber header "Line 3: Unknown node id 'Ghost' in @pos hint", but the block then shows the raw source (`flowchart TD` / `X[Only node]`) instead of a diagram. A scratch control without the bad line renders the node box. The warning is only in the UI, not in the log. |
| MMD-9 | PASS | shots/MMD-13-firstpaint-fit.png, shots/MMD-11-popup-native-100pct.png; logs/ferrite-debug-20261009-035014-MMD-1.log | FC-165a `subgraph 业务客户端 PEP`: full title, left-aligned, no ellipsis, in the popup at 100 % (Native) and inline (Fit, small). |
| MMD-10 | PASS | shots/MMD-10-partial-fc165b-inline.png, shots/MMD-10-partial-control-autonumber-10-10-and-off.png; logs/ferrite-debug-20261009-035014-MMD-1.log, logs/ferrite-debug-20261009-040706-MMD-10-control.log | FC-165b: badges 1–10 in source order, alt/else labels and participant aliases shown (PASS part). Control (scratch): `autonumber 10 10` → 10, 20, 30 (correct); `autonumber off` as the last directive removes **all** badges, so "off stops numbering from that point" is not implemented (the repo doc `docs/technical/mermaid/sequence-autonumber-rendering.md` declares mid-diagram off out of scope). Cosmetic: badges 5 and 6 sit on the message text. |
| MMD-11 | PARTIAL — ISS-32 (frames), ISS-8 (popup) | shots/MMD-11-inline-fc165a.png, shots/MMD-11-popup-native-100pct.png, shots/MMD-11-popup-68pct.png; logs/ferrite-debug-20261009-035014-MMD-1.log | FC-165a: no node-on-node overlap, `&` fan-outs in separate lanes, no edge through a node body, `<-->` arrowheads at both ends (checked at 100 % and 68 %). **But the 统一接入层 frame starts inside the 业务客户端 PEP frame**, its title sits inside it and hides the right border. Also: the expand icon did not open the popup in Fit mode (3 tries incl. Ctrl+click) but did on the first click after switching to Native width. |
| MMD-13 | **FAIL** — minor | shots/MMD-13-firstpaint-fit.png, shots/MMD-13-scrolled-away.png, shots/MMD-13-after-scroll-back.png, shots/MMD-11-popup-native-100pct.png; logs/ferrite-debug-20261009-035014-MMD-1.log | ISS-59. FC-165a (the only fixture with CJK node labels), popup 100 %: A-row boxes are 1.63–1.71× their text width (堡垒机运维系统 196 vs 116 px, SDP零信任访问 192 vs 112, AI Agent命令代理 212 vs 130); other boxes 1.04–1.36×, so sizing is inconsistent. Layout stable: first paint vs scroll-away-and-back differ in 26 of 748,000 px (scrollbar edge). |

#### Batch 12a — §7–§9 HTML, workspace, CSV, tables, UX

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| HTML-4 | PASS | shots/HTML-4-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | `<sub>`/`<sup>` lowered/raised correctly; `Line one<br>Line two<br>Line three` → three lines in one paragraph; a standalone `<br>` gives a blank gap. Side note: heading `## Phase 1 — <br> line breaks` shows a box glyph (□), and `## Phase 2 — <kbd>` drops the `<kbd>` text. |
| HTML-5 | PASS | shots/HTML-5-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | `<img>` 120×40 and 400×200 render at those sizes; `alt`/`title`-only renders at natural 64×64 (pixel-checked on the downscaled screenshot: 79×33, 266×164, 42×52 vs expected 78.75×32.5, 262.5×162.5, 42×52). |
| HTML-6 | **FAIL** — major (= ISS-33) | shots/HTML-6-fail.png; logs/ferrite-debug-20261009-041820-WS-1.log, logs/batch12/HTML-6-tail.txt | Same defect as T3-44: both single-line `<div align>` blocks (lines 42 and 44) show the document's first line `A# GitHub HTML Subset Test`, left-aligned; their own text never appears. The paragraph after them and all multi-line `<div align>` blocks are fine. The single-line `<div onclick>` in the Unsafe section shows only an «HTML» indicator (acceptable per the fixture; nothing executed). |
| WS-1 | PASS | shots/WS-1-hover.png; logs/ferrite-debug-20261009-041820-WS-1.log | Repo folder as workspace (50 earlier tabs restored silently from the shared session). File-tree hover highlight + tooltip; the active tab's row has a light-blue background and a bold blue name, and the emphasis follows the active tab. |
| WS-2 | PASS | shots/WS-2-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | Document panel › Stats › Runtime Modules: Loaded fonts = Emoji, Mermaid cache 0/0, Terminal sessions 0, Language Server Disabled (status bar `LSP: Disabled`). |
| CSV-4 | PASS | shots/CSV-4-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | `very_large_test_data.csv` (85.7 MB, 500,001 rows) opened lazily, responsive (RSS ~500 MB), grid via the R/S/V control (Ctrl+E did nothing on this file in Raw): "500001 rows total (lazy-parsed)". Cell edit refused with the banner "File is large (85.7 MB). Cell editing is disabled — switch to Raw view (Ctrl+E) to edit." A second red banner says "Showing raw content" while the grid is visible (wording mismatch, cosmetic). File unchanged. |
| CSV-5 | PARTIAL (fixture has no long values) | shots/CSV-5-csv-copy-pass.png, shots/CSV-5-tsv-copy-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | `test_data.csv`/`.tsv` have no long values (max 19/20 chars), so nothing can clip. Scratch copies outside the repo (`/tmp/csv5/long_values.csv/.tsv`, ~130–190-char values): long cells are clipped with an ellipsis at the column border, no overflow into neighbours, in both CSV and TSV grids. |
| TBL-2 | PARTIAL — ISS-41 | shots/TBL-2-pass.png, shots/TBL-2-rendered-before.png, shots/TBL-2-after-edit.png, shots/TBL-2-raw-after.png; logs/ferrite-debug-20261009-041820-WS-1.log | Copy `/tmp/tbl2.md`. Align buttons set column 1 left, 2 center, 3 right → after Ctrl+S the delimiter row is `\|:----\|:----:\|----:\|` (all three forms round-trip: PASS). **Cell edit:** typed `X` into "More" (rendered as `MorXe`), then switched to Raw and pressed Ctrl+S: Raw and the saved file still had `More`; the `X` reached Raw/disk only after returning to Rendered and clicking another cell (commit on focus loss only). Cosmetic: a cell in edit mode is left-aligned even in a centred column. |
| TBL-3 | PARTIAL — ISS-61 (+ ISS-62) | shots/TBL-3-tables-raw.png, shots/TBL-3-alignment-raw.png, shots/TBL-3-tables-nowrap.png, shots/TBL-3-guides-nowrap.png; logs/ferrite-debug-20261009-041820-WS-1.log | Viewing never dirties the doc (`git diff --stat` empty, no `*`): PASS. Guides: with word wrap ON (default) none are drawn (`table_guides.rs`: `if wrap_enabled { return; }`); with wrap OFF they follow the **header** pipes only, body pipes sit ~20 px left and the guides cut through body text (`test_tables.md`, `alignment_test.md`). Side effect: Alt+Z in the focused Raw editor toggled wrap **and** inserted a literal `z` at line 1 (undone; ISS-62). Word wrap restored to ON. |
| UX-4 | PASS | shots/UX-4-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | Outline (Ctrl+Shift+O): guide lines, per-type coloured bullets, collapse chevrons; the current-section pill follows the caret (H1 → Section 3 after clicking it; caret Ln 27); hover tooltips like "Heading 2 · Section 4 · line 44". |
| UX-7 | PASS | shots/UX-7-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log | Tab A → tab B → ribbon back-to-previous-tab arrow → back on A (title and tab highlight). Repeated once for the screenshot. |
| UX-8 | **FAIL** — minor | shots/UX-8-clicked.png, shots/UX-8-fail.png; logs/ferrite-debug-20261009-041820-WS-1.log | ISS-52. Click-to-edit on plain text and at line start is accurate. A click on `tail` after bold/italic/code/link markup on the same line put the caret about 10 source characters to the right ("after 'text '" per the tester's note). Reproduced on two `/tmp` copies. |
| UX-9 | **FAIL (hang)** — blocker (= ISS-1) + ISS-42 | shots/UX-9-fail.png, shots/UX-9-fail-2.png; logs/UX-9-hang-bt.txt, logs/UX-9-fresh-2-hang-bt.txt; logs/ferrite-debug-20261009-041820-WS-1.log, logs/ferrite-debug-20261009-044244-UX-9-fresh.log | A plain click on a file link switched the list item to source, then Ferrite deadlocked (`arrow_nav_sel_key ← remember_arrow_nav_sel ← render_session_formatted_edit_text ← render_list`); reproduced in a fresh instance. Ctrl+click and middle-click on file, folder and web links opened nothing: no thunar / xdg-open / browser process, no log line (ISS-42). Ignored schemes not tested. |
| UX-11 | PARTIAL — ISS-53 | shots/UX-11-partial.png; logs/ferrite-debug-20261009-041820-WS-1.log | F1 › About/Help › Shortcuts lists `Ctrl+Shift+R` Reload from Disk and `Alt+Z` Toggle Word Wrap: PASS. The ribbon "MD" Markdown quick reference lists only formatting entries (Bold … Task list) and **neither** shortcut. |

#### Batch 12b — §10–§11 code run, git, title bar (Linux), multi-window

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| RUN-1 | PARTIAL — ISS-55 | shots/RUN-1-consent-pass.png; logs/ferrite-debug-20261009-044740-RUN-1.log (no code-run lines at debug level) | Consent modal "Allow running code from this document?" with the no-sandbox warning and Enable & run / Just enable / Cancel: PASS. §2c Python → `Hello from Python`, "Exit 0 · 117ms": PASS. §10 `time.sleep(60)` → "Timed out after 30s" (default 30 s; the fixture's 5 s was not applied): PASS. §11 Stop at ~9.8 s → "Stopped by user", Run re-enabled: PASS. **But no partial stdout after Stop or timeout** (§11 expects `Running…`/ticks): (no stdout or stderr). `code_execution.rs` sets no `PYTHONUNBUFFERED`. Consent flag temporarily reset in config and restored afterwards. |
| RUN-4 | PASS | shots/RUN-4-pass-sleep12.png, shots/RUN-4-pass.png, shots/RUN-4-after-output.png; logs/ferrite-debug-20261009-044740-RUN-1.log, logs/ferrite-debug-20261009-045149-RUN-4.log | Scratch `/tmp/batch12b/run4.md` (`time.sleep(12)` first): "Running · 3.66s", Stop button and placeholder **"Waiting for output…"**; at 12.03 s the output replaced it ("Exit 0 · 12.03s"). |
| RUN-5 | PASS | shots/RUN-5-pass.png; logs/ferrite-debug-20261009-044740-RUN-1.log, logs/ferrite-debug-20261009-045051-RUN-5-insert-copy.log | §4: stdout lines, then a "── stderr ──" heading and `stderr line` ("Exit 0 · 21ms"). Copy → clipboard has the same 4 lines, no ANSI codes. Insert as block (on a scratch copy) → an `output` fence with the same text right below the Python fence. Not saved. |
| GIT-2 | PASS | shots/GIT-2-pass.png; logs/ferrite-debug-20261009-045325-GIT-2.log | Workspace = repo (git badge, branch `0.3.1-experimental`). `echo hi > test_md/zz_untracked.md` → the tree shows it with the untracked badge (green, `?`) without a refocus; `git status` agrees. |
| GIT-3 | PARTIAL (refocus not exercised) | shots/GIT-2-pass.png, shots/GIT-3-modified-before-refocus.png, shots/GIT-3-pass.png; logs/ferrite-debug-20261009-045325-GIT-2.log | Saved untracked file keeps `?`; tracked `wikilink-target-a.md` edit + save → orange modified dot (= ` M`); no `\\?\` anywhere (0 in the log). Cleanup (`git checkout`, `rm`) → badges and file gone from the tree. **Refocus step not exercised:** the window never lost focus (Alt+Tab did not switch). Observation: each save of the untracked file logged `Reloaded externally modified file` right after `Saved file`; one save during typing wrote `hi`/`edi` and the buffer reloaded before the rest of the typing arrived (likely a harness-timing effect; see observations). |
| TTL-6a | **FAIL** — minor (brief §5) | shots/TTL-6-a-dblclick-restore.png; logs/ferrite-debug-20261009-045325-GIT-2.log (no geometry lines) | ISS-54. Custom title bar, geometry via `xdotool getwindowgeometry` (read-only). Double-click maximize → double-click restore: the icon flipped but the window stayed 1920×1200 at x=−232 (left part off-screen); a further double-click did not maximize. Maximize button works. Drag of the restored window: PASS (+120/+85 px). **Drag from maximized: no restore** (2 tries). Edge-resize not achieved (the drag landed on another terminal window). Slow multi-point drags were needed. |
| TTL-6b | PARTIAL — ISS-37 | shots/TTL-6b-on-native-titlebar.png, shots/TTL-6b-pass.png; logs/ferrite-debug-20261009-050309-TTL-6b.log, logs/ferrite-debug-20261009-050403-TTL-6b-on.log, logs/ferrite-debug-20261009-050735-TTL-6b-off.log | **ON (via UI):** Appearance › *Use system title bar* ("Restart Ferrite for this change to take effect."), saved at quit; after relaunch xfwm4 decorations replace the custom bar; double-click and maximize respond (drag and edge-resize not recorded separately). **OFF could not be done through the UI:** with the system title bar there is no gear, and `Keyboard shortcut: Ctrl+, (Open Settings)` was logged 3 times without opening Settings (ISS-37). Done via `config.json` (`"use_system_title_bar": false`, Ferrite closed); the custom bar came back and Settings shows the box unticked. |
| MW-5 | SKIPPED (no v1 session fixture) | none | `test_md` and `fixtures` contain no v1 session file; the only session files are the live `session.json`/`session.recovery.json` (`"version": 2`). Nothing was hand-crafted (gui-test-plan rule). |

#### Total-test rows not run
Skipped by the platform rule (brief §5) or optional. They are counted in the summary as SKIPPED (platform) and NOT RUN.

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| TTL-1 | SKIPPED (platform) | — | Windows custom title bar (brief §5). Linux sanity is TTL-6a. |
| TTL-2 | SKIPPED (platform) | — | Windows (brief §5). |
| TTL-3 | SKIPPED (platform) | — | Windows, Win+↓ (brief §5). |
| TTL-4 | SKIPPED (platform) | — | Windows (brief §5). |
| TTL-5 | SKIPPED (platform) | — | Windows multi-monitor (brief §5). |
| MAC-1 | SKIPPED (platform) | — | macOS. |
| MAC-2 | SKIPPED (platform) | — | macOS. |
| MAC-3 | SKIPPED (platform) | — | macOS. |
| MAC-4 | SKIPPED (platform) | — | macOS. |
| PLT-1 | SKIPPED (platform) | — | Windows Inno Setup installer. |
| PLT-3 | SKIPPED (platform) | — | Windows hardware carry-over rows (WIN-8, KBD-8, KBD-9). |
| RUN-2 | SKIPPED (platform) | — | `(Windows)` bash fence without Git Bash. On Linux bash fences run normally (brief §4). |
| GIT-1 | SKIPPED (platform) | — | Windows verbatim `\\?\` paths. GIT-3 confirmed no `\\?\` text appears on Linux. |
| STAT-4 | NOT RUN (optional) | — | Needs a German keyboard layout with dead keys; not set up on this box. |

### EXTRA rows from gui-test-plan.md (Ola's focus areas: Settings, YouTube; EXTRA-YT-7 and EXTRA-SET-5/6/7/13/14 run in Batch 12)
| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| EXTRA-YT-1 | PARTIAL | shots/VID-4-cases.png, shots/VID-4-pass.png; `…-223520-VID-1.log`, `…-223312-VID-4.log` | A1 (`watch?v=`) **FAIL (crash)**. A2–A14, C5–C9, D1–D8 BLOCKED (trusted URLs → crash). **B1–B5** (nocookie, no id, playlist-only, `youtube.com.evil.example`, vimeo) → placeholder, **no WebView: PASS**. **C1–C4** (`javascript:`, `file:///`, `notaurl`, `{{video}}`) → literal text, no embed: PASS (C1/C4 also show a grey hatched bar). |
| EXTRA-YT-2 | **FAIL (crash)** | shots/EXTRA-YT-2-fail-offline-crash.png; `…-224251-EXTRA-YT-2.log` | Offline (`unshare -n`). Rendered aborted (exit 134, 22:43:28). The thumbnail fetch failed with DNS error and was logged as WARN; 1 thumbnail-fetch WARN, no `ureq::` lines, no retry storm before the abort. The placeholder, error-on-play, and retry-storm checks were unreachable. |
| EXTRA-YT-3 | BLOCKED (crash) | — | No player ever rendered. |
| EXTRA-YT-4 | **FAIL (crash)** | shots/EXTRA-YT-4-fail-crash.png; `…-223923-EXTRA-YT-4.log` | `yt-many.md` (12 embeds): RSS 207,088 KB in Raw. Rendered aborted (exit 134, 22:39:37). |
| EXTRA-YT-5 | BLOCKED (crash) | — | Entering Rendered aborts. |
| EXTRA-YT-6 | **FAIL (crash)** | shots/EXTRA-YT-6-live1.png, shots/EXTRA-YT-6-live2.png, shots/EXTRA-YT-6-fail-crash.png; `…-223957-EXTRA-YT-6.log` | In Split, typing `{{video https://youtu.be/dQw4w9WgXcQ width=320}}` character by character aborted (exit 134, 22:42:22) on the keystroke that completed a trusted embed. The vimeo placeholder in the same doc was fine. |
| EXTRA-YT-7 | **FAIL (crash)** — = ISS-2 | shots/EXTRA-YT-7-fail.png; logs/ferrite-debug-20261009-045325-GIT-2.log, logs/ferrite-debug-20261009-050216-EXTRA-YT-7-fresh.log | Batch 12b. `/tmp/batch12b/yt7.md` (copy of `test_md/test_video_embeds.md`, embeds #1–#4) opened in Raw, then Rendered → immediate panic `GDK has not been initialized. Call gdk::init or gtk::init first.` (`gdk-0.18.2/src/auto/display.rs:301`, via `create_child_webview` ← `render_video_embed`), exit 134. Reproduced 2× (05:01:39 in the GIT-2 instance, 05:02:39 in a fresh one). Players, Settings overlay, reload dialog and Search-in-Files over players were **not reached**; nothing is claimed for them. |
| EXTRA-YT-8 | BLOCKED (crash) | — | Needs a visible player. |
| EXTRA-SET-1 | PARTIAL | none saved; `…-221729-EXTRA-SET-10.log` | Ribbon gear opens Settings, and clicking the gear again *closes* it (toggle). Tab × closes it. Switching tabs keeps it, and there were no duplicate tabs. **Esc does not close Settings** (1 attempt). **Ctrl+, did not open Settings** (several attempts across rows; possibly input injection). Command palette not reachable (Alt+Space = WM menu; Ctrl+Shift+P = Export PDF). ISS-12. |
| EXTRA-SET-2 | PASS (persistence) | shots/SET-2-pass-before-quit.png, shots/EXTRA-SET-2-pass-mocha-visible.png; `…-215656-set-extra.log`, `…-220315-EXTRA-SET-2-relaunch.log` | Theme, accent, font, rendered font, size 20, syntax theme, scripts, and CJK preference all persisted and matched config.json. config.json is written only on exit. After relaunch, Recently Changed lists "Ferrite accent". |
| EXTRA-SET-2-defect | **FAIL** | shots/EXTRA-SET-2-fail-ansi-invisible-text.png; `…-220315-EXTRA-SET-2-relaunch.log` | ISS-10. Dark theme + Syntax Theme **ANSI** (+ JetBrains Mono 20px): Raw editor body text is invisible (reproduced 2×). Switching to Base16 Mocha Dark fixes it. |
| EXTRA-SET-3 | PASS (persistence) | shots/EXTRA-SET-3-pass.png; `…-215656-set-extra.log`, `…-220823-EXTRA-SET-3-relaunch.log` | 19 Editor values flipped and persisted. Effects verified: line numbers hidden, syntax highlighting off, minimap shown, Vim `[NORMAL]` on/off. Word-wrap effect not verified. Checkbox repaint lag (ISS-24); stale `[NORMAL]` on the Settings tab (ISS-25). |
| EXTRA-SET-4 | **FAIL** (final, after restart) | shots/set-persist-recent.png; `…-222853-set-persist.log` | Restore session OFF, welcome OFF, quick note OFF, and Auto-Save ON 15 s all persisted (log: "Session restore disabled in settings"). **Recent Files 0 was not honoured**: config.json had `max_recent_files=0` before launch, but the slider read "Remember last 10 files" after relaunch. ISS-11. |
| EXTRA-SET-5 | PARTIAL — ISS-43 (major) | shots/EXTRA-SET-5-fail.png; logs/ferrite-debug-20261009-050949-EXTRA-SET-5.log, logs/ferrite-debug-20261009-051246-EXTRA-SET-5-relaunch.log | Batch 12c. All 10 Terminal settings changed in one instance (font size 32 = slider max, font "Abel", scrollback 50000, Copy Selection ON, Dracula, opacity 10 % = slider min, breathing colour yellow, Auto-load Layout OFF, sound ON, focus-on-prompt ON). Clean exit; `config.json` valid JSON with all 10 values; after relaunch all 10 shown and the terminal works (faint but readable at 10 %). **Defect:** Auto-load Layout OFF did not stop the workspace layout from loading (03:13:18Z), ISS-43. Only the upper extremes were tried. Side note: Recently Changed keeps 8 entries. |
| EXTRA-SET-6 | PARTIAL (Ctrl+Alt+Shift sub-step BLOCKED, input) | shots/EXTRA-SET-6-defaults.png; logs/ferrite-debug-20261009-051502-EXTRA-SET-6.log, logs/ferrite-debug-20261009-052055-EXTRA-SET-6-relaunch.log, logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log | Batch 12c. Outline→F7, Word Wrap→F8, Terminal→Ctrl+Shift+J survived Alt+F4 + relaunch and each key fired its command (03:21:08Z, 03:21:15Z, 03:21:44Z; Ctrl+Shift+J needed 3 tries). Esc cancels capture; a modifier-only Shift does not bind. **Ctrl+Alt+Shift combos BLOCKED** after 4 tries: captured as `0`, as bare `K` (log 03:17:23Z: `K (Toggle Outline)` and `Ctrl+Shift+K (Image)` fired from the same press, so Alt never reached Ferrite), or capture stuck at "Waiting for key…". Most likely xdotool/xfwm4 dropping Alt; not verifiable from Ferrite's side. **Reset All Shortcuts** restored the defaults in the UI and in `config.json` (`{"bindings": {}}`), still defaults after relaunch. |
| EXTRA-SET-7 | PARTIAL — ISS-56 (minor, triage) | shots/EXTRA-SET-7-zh_Hans.png, shots/EXTRA-SET-7-ja.png; logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log, logs/ferrite-debug-20261009-052638-EXTRA-SET-7-ja-restart.log | Batch 12c. The dropdown offers only English, Chinese (Simplified), German, Spanish, Japanese; cs, et, nb_NO, pt, ru are not offered although `locales/` has them (so no ru screenshot). The 5 offered apply live, no raw keys, no tofu, no clipping seen. Japanese: several Keyboard labels stay English. All non-English UIs: "Use system title bar" and its restart hint stay English. Japanese persisted across a restart (`language: ja`); switching back to English works. WARN `Failed to find replacement characters` at Japanese font load (03:26:07Z), no visible glyph loss. |
| EXTRA-SET-8 | PARTIAL | none saved; `…-220823-EXTRA-SET-3-relaunch.log` | Max Line Width clamps correctly: 0 → 400, -5 → 400, 99999 → 2000, `abc` ignored. **No validation or feedback** for startup command (`;;;`, `$(rm -rf nowhere)` stored as text only, never executed), invalid regex `([`, or sound path `/nonexistent.wav` (ISS-19). Font fields are dropdowns and Tab Size is a slider, so they can't be typed into. LSP path hidden; no code-exec working-dir field found. No crash. |
| EXTRA-SET-9 | PASS (row criteria: no crash/hang) — but see ISS-9 | shots/EXTRA-SET-9a-pass.png; `…-221551-EXTRA-SET-9a.log`, `…-221645-EXTRA-SET-9b.log` | (a) `{ not json` and (b) `font_size: "abc"`: the app starts with **all defaults**, shows only a WARN in the log, and offers no toast or `.bak` restore. On clean exit it **overwrites config.json with defaults**. One bad field discards the whole config. |
| EXTRA-SET-10 | PASS | shots/EXTRA-SET-10-pass.png; `…-221729-EXTRA-SET-10.log` | `[*`, `\`, `.`, `中文`, 🙂, ` zoom `, `ZOOM`, and a 200-char query: sensible results, case-insensitive, no freeze. The 🙂 glyph is invisible in the field (ISS-29). The first char was dropped once (ISS-18). |
| EXTRA-SET-11 | PASS (after restart) | shots/set-persist-recent.png; `…-221729-EXTRA-SET-10.log`, `…-222853-set-persist.log` | Theme toggled 13×, minimap 11×, font slider dragged back and forth. No crash. Final values (Light, 72px, minimap OFF) persisted. Terminal opacity not tested. |
| EXTRA-SET-12 | PARTIAL | shots/EXTRA-SET-12-pass.png; `…-221729-EXTRA-SET-10.log` | In a small window (about 330×270 by eye, maybe smaller than the 500×400 target) with search active, content scrolls and nothing crashes. The no-search state and scroll-to-bottom per chip were not verified. |
| EXTRA-SET-13 | PASS | shots/EXTRA-SET-13-restart-check.png; logs/ferrite-debug-20261009-052638-EXTRA-SET-7-ja-restart.log, logs/ferrite-debug-20261009-053015-EXTRA-SET-13-restart.log | Batch 12c. Ctrl+Shift+N opened window 2, but only on the 5th press (presses 1–4 left no shortcut line in the log). In window 2: Theme Light→Dark (03:29:07Z) and font 14→18 px; **window 1 updated live**, no restart (window 1's own Settings tab not inspected). After a full restart: dark, 18 px, `config.json` agrees. The tester toggled the title-bar Zen icon once by accident and reverted it. |
| EXTRA-SET-14 | PARTIAL — ISS-17 (minor) | shots/EXTRA-SET-14-defaults.png; logs/ferrite-debug-20261009-053015-EXTRA-SET-13-restart.log, logs/ferrite-debug-20261009-053158-EXTRA-SET-14-relaunch.log | Batch 12c. Settings › Reset All applied on **one click with no confirmation** (ISS-17); status "Settings reset to defaults"; every checked value back to default at once, and still default after a restart (`config.json` checked field by field, `keyboard_shortcuts {"bindings": {}}`). Recently Changed is not cleared in-session (8 entries stay, showing defaults); it is empty after quit. |

### Total-test rows covered by minimal-test rows (not re-run)
| Row | Result | Covered by |
|-----|--------|-----------|
| STAT-1, STAT-2 | PASS | T1-12 |
| EOL-1 | PASS | T1-1 |
| EOL-2 | **FAIL (hang)** | T1-2 (ISS-1) |
| EOL-4 | PASS | T1-3 |
| EOL-6, EOL-8 | PASS | T1-4 (TOC lines `0d0a`) |
| LOPS-1…3 | PASS | T1-13 |
| SES-2 | PASS | T1-10 |
| SES-7 | PASS | T1-11 |
| RLD-1 | PASS | T1-5 |
| RLD-2 | PASS | T1-6 |
| RLD-6 | PASS | T2-27 |
| RLD-8 | **FAIL (hang after dialog)** | T1-7 (ISS-1) |
| ARR-6 (first half) | **FAIL (hang)** | T1-17 (ISS-1) |
| UND-1 | **FAIL** | T1-19 (ISS-6) |
| UND-2 | PASS | T1-19 (SENTINEL lines intact) |
| IND-6, IND-10 | BLOCKED | T1-20 (Alt+click; Ctrl+click supplementary behaved correctly) |
| IMG-3, IMG-9 | **FAIL** | T1-14 (ISS-5; right-click Paste works, no crash after CJK) |
| IMG-8 | **FAIL** | IMG-X1 (ISS-5 + ISS-46; Batch 7) |
| MMD-8 | **FAIL** | T1-22 (ISS-8) |
| MMD-12 | **FAIL** (no crash) | T1-21 (ISS-7) |
| UX-6 | PASS | T2-27 |
| ENT-1…5 | PASS | T2-31 |
| FIND-1…4 | PASS | T2-30 |
| ARR-3 | **FAIL (hang)** | T2-35 (ISS-31 = ISS-1; edits were correctly blocked, but arrow keys froze the app) |
| CPY-1 | BLOCKED | T2-34 (synthetic drag-select failed; the Copy-button path gave byte-identical output) |
| EMO-1, EMO-2 | **FAIL** | T2-32 (ISS-30: monochrome, 2 of 7 common emoji blank in Raw and Rendered) |
| EMO-3 | PASS | T2-32 (toolbar and tab icons unchanged) |
| IMG-1, IMG-6 | PASS | T2-33 |
| MMD-5 | PASS | T3-43 (GG-01…03 render with no overlap; labels ellipsised) |
| MMD-7 | **FAIL** | T3-43 (ISS-32) |
| MW-1…4 | PASS | T2-37 (window geometry was not compared) |
| MW-6 | PARTIAL | T2-38 PASS (raised when it was the only Ferrite window); SI-1 **FAIL** (not raised with another Ferrite window on top, ISS-34) |
| HTML-1 | **FAIL** | T3-44 (multi-line divs OK; adjacent single-line divs not rendered, ISS-33) |
| HTML-2 | PARTIAL | T3-44 (collapse/expand OK; `<strong>` in a summary shows as tag chips, not bold) |
| HTML-3 | PASS | T3-44 |
| TBL-1 | PASS (supplementary fixture) | T3-41 (`test_table_wrapping.md`; the named fixture has no `:-:`/`--:` column) |
| TBL-4 | PARTIAL | T3-41 (cell edit commits; preservation of formatted cells not checked; `[**bold link**](url)` shows literal `**`, ISS-35) |
| CSV-1 | PARTIAL | T3-42 (double-click edit OK; Enter commit and Esc cancel not exercised) |
| CSV-2 | PARTIAL | T3-42 (Tab moves to the next cell; Shift+Tab and arrow clamping not exercised) |
| CSV-3 | PASS | T3-42 (Ctrl+Z reverts the cell edit) |
| UX-1 | PASS | T2-35 (lock blocks Rendered typing, Raw stays editable, persists across restart; CSV/Tree under the lock not tested) |
| UX-2 | BLOCKED | T2-39 (Alt+Z unverified with synthetic input; palette entry and 100K-line toast not tested) |
| UX-3 | PASS | T2-39 (New/Close/Copy Path/Reveal, drag-reorder; path hover tooltip not checked) |
| UX-5 | PASS | T3-46 |
| UX-10 | PARTIAL | T3-47 (zip/exe handed to xdg-open; toast not observed) |
| RUN-3 | PASS | T3-45 |

## Results — §6 extra rows
SI-1…5 were run in Batch 6b (Oct 8–9). SIF, PRI, SPL, FNT and IMG-X were run in Batches 7–8 (Oct 9, 01:29–02:28 CEST; `results-batch7.md`, `results-batch8.md`).

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| SIF-1 | **FAIL** — = ISS-4 (blocker, must-pass row 15) | shots/SIF-1-fail.png, shots/SIF-1-zoom.png, shots/SIF-1-retry2-zoom.png; logs/ferrite-debug-20261009-012958-SIF-1.log | Line 2 (`Istanbul İstanbul istanbul`) was appended to `test_md/zz_cjk_search.md` first. Ctrl+Shift+F `中文` → "60 matches in 26 files", one hit in the fixture. Clicking it: the log has the right target (`at line 1, char offset 5`, `Set transient highlight at 5..7`), then `after set_cursor, cursor_after=(0,0)` and `cursor sync changed: (0, 5) -> (0, 0)`. Status bar Ln 1, Col 1 (expected Col 6). Reproduced 3 times. No crash, no hang. The input tool dropped or mis-sent Ctrl+Shift+F several times; the stray edits were undone and the file was never saved. |
| SIF-2 | **FAIL** — = ISS-4 | shots/SIF-2-fail.png, shots/SIF-2-zoom.png, shots/SIF-2-statusbar.png, shots/SIF-2-recheck-zoom.png; logs/ferrite-debug-20261009-013635-SIF-2.log | Query `search` (pasted from the clipboard, because typed input dropped characters). Log `char offset 8` (correct after the CJK text), then the caret reset `(0, 8) -> (0, 0)`; status Ln 1, Col 1 (expected Col 9). A first-run claim of a 3-char highlight was **retracted** (it came from a mistyped query). On the re-check the fixture's hit was outside the 50-file result cap, so the editor-side match length was not re-verified. No crash. |
| SIF-3 | **FAIL** — ISS-44 (`İ` case folding) + ISS-4 (caret) | shots/SIF-3-query.png, shots/SIF-3-click1.png, shots/SIF-3-click2-fail.png, shots/SIF-3-Idotted-query.png, shots/SIF-3-click-Idotted-status.png; logs/ferrite-debug-20261009-014003-SIF-3.log | Case-insensitive `istanbul` finds only 2 of 3 variants: `Istanbul` (char 15) and `istanbul` (char 33). **`İstanbul` (U+0130, char 24) is not found**; the exact query `İstanbul` finds it. Clicks: `Istanbul` → Ln 2, Col 1 = correct word start; `istanbul` → highlight 33..41 correct, but caret reset `(1, 18) -> (1, 0)` (expected Col 19); `İstanbul` → highlight 24..32 correct, caret reset `(1, 9) -> (1, 0)` (expected Col 10). The offsets after `İ` are right (no byte/char drift); the caret reset is ISS-4. One click on stale results was discarded. No crash. |
| SIF-4 | PASS | shots/SIF-4-pass-zz-find.png, shots/SIF-4-t1-zoom.png, shots/SIF-4-pass-t1-eacute-whole-case.png; logs/ferrite-debug-20261009-015551-SIF-4.log | Fresh instance, Raw. Ctrl+F, Whole word, `中文` → 1 of 1, highlight on 中文. Aa + Whole word, `é` on `fixtures/t1-multibyte.md` line 7 (`éa é`) → 1 of 1, only the standalone `é`. No crash; doc not modified. Side observation: clicks on the Find panel's toggles also reached the tab strip underneath and switched tabs (keyboard input was reliable). |
| SI-1 | **FAIL** — minor | shots/SI-1-fail.png; `…-235629-SI-1.log` | ISS-34. `time $FER /tmp/lf-test.md`: 0.095 s, rc 0, no 2nd process left. The primary logged "File already open, switching to window 0 tab 3", with no duplicate tab (tab part PASS). With the other Ferrite window (test_wrap_debug.md) on top, the lf-test.md window was **not raised**; the log shows only `request_user_attention` (Informational) ×2. Reproduced once more (0.080 s, same result). In T2-38 the window was raised, but it was the only Ferrite window then. |
| SI-2 | PASS | shots/SI-2-pass.png; `…-235629-SI-1.log` | Closed the tab, then `cd /tmp && time $FER ./lf-test.md`: 0.087 s, rc 0. The relative path resolved and the tab reopened with the fixture content; no "file not found". |
| SI-3 | PASS | shots/SI-3-pass.png, shots/SI-3-pass-closed.png; `…-235629-SI-1.log`, `…-235858-SI-3.log` | `--new-instance` gave a 2nd process and window. `instance.lock`/`.port`/`.pid` kept the primary's timestamps, and `instance.pid` = primary PID before and after the 2nd instance quit (exit 0; primary still running). Side findings for triage: the 2nd instance treated the primary's session.lock as stale, and on exit removed it and overwrote session.json/config.json (see observations). |
| SI-4 | PASS | shots/SI-4-setting-on.png, shots/SI-4-setting-persisted.png; `…-235629-SI-1.log`, logs/si4-secondary-ON-stderr.out | Settings › Files › "Allow multiple Ferrite windows/instances" is off by default. ON: a second launch spawned a 2nd process (no forwarding), and closing it left the primary running. OFF: forwarding came back (1 process, "File already open, switching…", 0.119 s). The toggle reached config.json only at a later save event (see observations). Settings was opened with the gear because Ctrl+, again did nothing. |
| SI-5 | PASS | shots/SI-5-pass.png; logs/ferrite-debug-20261009-000447-SI-5.log, `…-235629-SI-1.log` (killed primary, exit 137), logs/launcher-SI-5.out | `kill -9` on the primary left stale instance.* files. The next launch started over them with no hang and became the new primary (instance.pid and instance.port updated). It showed Session Recovery (answered Start Fresh). Forward check: 0.089 s, rc 0, 1 process. |
| PRI-1 | PASS | shots/PRI-1-pass.png; logs/ferrite-debug-20261009-015853-PRI-1.log | Raw: PRIMARY set to a sentinel, then double-click on `search` → after 1 s `xclip -selection primary -o` = `search`. |
| PRI-2 | PASS | shots/PRI-2-pass-inserted.png, shots/PRI-2-zoom.png; logs/ferrite-debug-20261009-015853-PRI-1.log | `printf 'PRIMARY-TEXT' \| xclip -selection primary`, middle-click inside line 2 → inserted at the click position (42 → 54 chars). One Ctrl+Z removed all of it (`Undo (0 remaining)`). Nothing saved. The log has no paste-specific line; evidence is the screenshot and status bar. |
| PRI-3 | **FAIL** — minor | shots/PRI-3-fail-tab-still-open.png; logs/ferrite-debug-20261009-015853-PRI-1.log | ISS-45. Middle-click on 4 different tabs (one of them twice, after hovering): none closed, no tab-close line in the log, no crash. Middle-click works in the editor (PRI-2). The older `shots/PRI-3-pass.png` (an earlier attempt) also shows the tab still open; it is misnamed and is **not** evidence of a pass. |
| PRI-4 | PASS | shots/PRI-4-setting-off.png, shots/PRI-4-setting-zoom.png, shots/PRI-4-pass-paste-off-nothing.png; logs/ferrite-debug-20261009-015853-PRI-1.log | Settings › Editor › `Middle-click paste` OFF → the PRI-2 middle-click inserted nothing (42 chars, no `*`). Turned back ON and verified. |
| SPL-1 | PASS | none; logs/ferrite-debug-20261009-020610-SPL-1.log, logs/ferrite-debug-20261009-021724-SPL-4.log | Default OFF (pre-run config `spellcheck_enabled: false`, checkbox unticked). Turned ON. The transient "Loading dictionary…" text was **not seen**: the first pass did not record it, and a re-check about 0.5 s after the toggle showed none (the dictionary may already have been loaded in that session). Noted, not verified. |
| SPL-2 | **FAIL** — minor | shots/SPL-2-fail.png (same image as `SPL-2-pass.png`, misnamed at capture); logs/ferrite-debug-20261009-020610-SPL-1.log | ISS-47. Pasted `This is a speling mistaek in Ferrite.` into a new Untitled doc (Raw). `speling` squiggled (tooltip "[hint/spell] Unknown word: speling"); **`mistaek` not squiggled** (zoomed; it was not in user-words.txt yet). `This` and `is` not squiggled; `Ferrite` unclear (tooltip covers it). |
| SPL-3 | BLOCKED (tooltip over context menu / input) | shots/SPL-3-check.png; logs/ferrite-debug-20261009-020610-SPL-1.log | Right-click `speling`: the "[hint/spell] Unknown word" tooltip overlapped the context menu. Clicks on the suggestion fell through (caret placed) or closed the menu; one stray click ran the menu's Paste (undone with Ctrl+Z); Down + Enter did not select. About 4 attempts. Replacement and the one-step Ctrl+Z were **not** verified. |
| SPL-4 | PARTIAL | shots/SPL-4-pass-relaunch.png; logs/ferrite-debug-20261009-021724-SPL-4.log | `mistaek` was written to `~/.config/ferrite/spellcheck/user-words.txt` at 02:07:22 during the SPL-3 attempts, but the tester could not pin down a deliberate *Add word* click, so "squiggle disappears immediately" is unverified. Persistence: after Alt+F4 (Don't Save) and relaunch, `mistaek` is not flagged while `speling` and `Ferrite` are; the file contains `mistaek` (one line). |
| SPL-5 | **FAIL** — minor | shots/SPL-5-fail.png (same image as `SPL-5-pass.png`, misnamed at capture); logs/ferrite-debug-20261009-020610-SPL-1.log | ISS-47. No squiggle on the fenced `speling`, inline-code `speling`, `https://exmaple.com/pth`, `camelCaseWord`, `ALLCAPS`, `abc123` or the link destination. **But `ok` in the link text `[ok](http://speling.com)` is squiggled** in Raw. |
| SPL-6 | PASS | shots/SPL-6-rendered-pass.png, shots/SPL-6-split-pass.png; logs/ferrite-debug-20261009-020610-SPL-1.log | Split (Ctrl+E) and Rendered: no squiggles in the rendered pane; `ok` is a normal blue link. The Raw pane in Split keeps the `ok` squiggle (SPL-5). |
| SPL-7 | PARTIAL | shots/SPL-7-before-scroll.png, shots/SPL-7-scroll1.png, shots/SPL-7-scroll2.png; logs/ferrite-debug-20261009-020610-SPL-1.log | `test_scroll_perf.md`, Raw, wheel scroll: no freeze, no visible stutter (an impression only; software GL, not measured). Squiggles only in the viewport. **But the squiggles are drawn at the far right edge of the text column, about one wrap-width away from the word (e.g. `viewport`), not under it** (ISS-48). |
| SPL-8 | PASS | shots/SPL-8-pass-off.png; logs/ferrite-debug-20261009-021724-SPL-4.log | Spellcheck OFF → squiggles gone at once, no crash. After relaunch, Recently Changed shows Spellcheck unchecked. |
| FNT-1 | PASS | shots/FNT-1-settings-untick.png, shots/FNT-1-settings-jbmono.png, shots/FNT-1-rendered-before.png, shots/FNT-1-rendered-after-jbmono.png, shots/FNT-1-raw-before.png, shots/FNT-1-raw-after.png; logs/ferrite-debug-20261009-021724-SPL-4.log | Default "Same as editor" (ticked). Unticked → built-in JetBrains Mono: the Rendered view of `fixtures/t1-multibyte.md` changed from proportional (Inter) to monospace; Raw unchanged (visual compare; a Settings transition overlay spoils a clean pixel diff). |
| FNT-2 | PASS | shots/FNT-2-dejavu-serif-set.png, shots/FNT-2-rendered-dejavu-serif.png; logs/ferrite-debug-20261009-021724-SPL-4.log, logs/ferrite-debug-20261009-022442-FNT-2.log | Custom System Font `DejaVu Serif` applied immediately. After Alt+F4 (exit 0) and relaunch still selected, pane still serif, config `rendered_font_family: {"custom": "DejaVu Serif"}`. Set back to "Same as editor". |
| FNT-3 | PASS | shots/FNT-3-dejavu-sans-mono-set.png, shots/FNT-3-dejavu-sans-mono-terminal.png, shots/FNT-3-jbmono-terminal.png; logs/ferrite-debug-20261009-022442-FNT-2.log | Default "Default (JetBrains Mono)". `DejaVu Sans Mono`: the terminal panel redraws in it, `ls -la` columns aligned, `echo gjpqy` descenders not clipped. Set back to default (config `terminal_font_family: null`). |
| FNT-4 | PARTIAL | shots/FNT-4-nerdfont-pua.png, shots/FNT-4-nerdfont-octal.png, shots/FNT-4-nerdfont-python.png; logs/ferrite-debug-20261009-022442-FNT-2.log | The Ferrite terminal runs `/bin/sh` (dash), whose `printf` does not expand `\u`, so the brief's exact command printed literal text. Octal `printf` and `python3 -c` did emit the code points: a powerline arrow glyph, a tofu box (U+F015) and an icon glyph (U+E62B). No toast, no crash; repeated non-fatal epaint WARN "Failed to find replacement characters". The optional Nerd Font install was skipped. |
| IMG-X1 | **FAIL** — = ISS-5 (+ ISS-46) | none (recorded from the operator's notes and the log after the session ran out of steps); logs/ferrite-debug-20261009-020219-IMG-X1.log | Untitled tab, PNG on the clipboard (`xclip -selection clipboard -t image/png -i …/sample.png`), Ctrl+V twice: **no toast** and nothing inserted. Log `arboard paste error: … incorrect type received from clipboard` at 00:02:52Z and 00:03:06Z (as in T1-14), so the save-first path is never reached. Good part: no `assets/` in the launcher's cwd, and `ferrite/assets/` holds only its pre-existing entries. |
| IMG-X2 | **FAIL** — = ISS-5 | shots/IMG-X2-fail.png; logs/ferrite-debug-20261009-020435-IMG-X2.log | Saved doc `fixtures/t1-multibyte.md` (md5 unchanged), clipboard TARGETS = `image/png` only. Ctrl+V twice quickly, then once more: every paste fails with the same arboard error (00:04:57Z ×2, 00:05:16Z). No new PNG in `fixtures/assets/`, nothing inserted, no toast, exit 0. The two-files / no-overwrite case could not be exercised. |

## Issues found
Severity follows the brief: Tier 1 FAILs are `blocker` (brief §8). Where the intrinsic severity (crash / major / minor) differs, it is given in brackets. Log excerpts ("last ~30 non-span lines" = `grep -v 'tracing::span' LOG | tail -n 30`, plus targeted excerpts around the failure) are in **Log excerpts per issue** below the table. Lines longer than 300 chars are truncated and marked. ISS-36 and later (Batches 7–12) are not Tier 1 rows, so they use the brief's severity guide directly (crash/hang/data loss = blocker). Duplicates were merged: every new freeze is ISS-1, EXTRA-YT-7 is ISS-2, HTML-6 is ISS-33. **Status** (round 2 close-out, tasks 1–33 + prior commits): `fixed` / `hardened` / `documented limitation` / `by design` entries mean code or docs landed on HEAD — **GrokBot re-run pending** until `grokbot/New/rerun-v0.3.1-round2.md` is green. Mermaid layout ISS-32 remains **`triaged — defer 0.3.2`** (task 15). ISS-49/ISS-50 remain **`triaged — closed (task 18; by design)`**. Tier B from round 1 (ISS-12/13/14/18/24/29) is tracked in `docs/technical/platform/grokbot-qa-tier-b-status.md`; this table still shows QA-commit Actual text — update Status when re-run confirms.

| ID | Rows | Severity | Status | Title | Exact repro steps | Expected | Actual | Evidence |
|----|------|----------|--------|-------|-------------------|----------|--------|----------|
| ISS-1 | T1-2, T1-7, T1-17, T1-18, T1-18b, T1-22 (session 1); = EOL-2, RLD-8, ARR-6; also T2-35 / ARR-3 (ISS-31); Batches 9–12: ARR-1, ARR-2, ARR-4, CPY-3, IND-9, UX-9 | **blocker** (hang, cannot save/exit) | **fixed (`5325e9b`) — GrokBot re-run pending** | Rendered/Split edit session self-deadlocks the UI thread (`arrow_nav_sel_key` → parking_lot `lock_shared_slow`) | **A (ArrowDown):** 1. Open `test_md/test_find_arrow_nav.md`. 2. Switch to **Rendered**. 3. Click the first heading. 4. Press **ArrowDown** once (froze on the 1st press in the retry; T1-18: froze on the 2nd press toward the blockquote).<br>**B (commit by click-away):** 1. Create the CRLF fixture from the minimal test (`# CRLF Doc / First paragraph. / ## Heading Two / …`). 2. Rendered → click "First paragraph." → type ` edited`. 3. Click a blank area below the content. (Same with `t1-18b.md`: edit the callout body and click away.)<br>**C (dialog):** 1. Rendered, click a paragraph, type 2 chars, don't click away. 2. Ctrl+Shift+R → Unsaved Changes dialog → **Cancel**.<br>**D:** `test_mermaid_issue_165.md` Rendered → click a paragraph (enters edit mode) → click the Mermaid expand icon.<br>**E (Batches 9–12):** Split view, click a paragraph in the preview pane, press ArrowUp or ArrowDown (ARR-4); or Rendered `test_md/test_local_links.md`, plain click on a file link inside a list item (UX-9). | Navigation or commit works; the app stays responsive; edits can be saved. | The whole UI freezes: no input handled, log silent (crash-recovery snapshots stop), process alive in `futex_wait`, no panic. Requires `kill -9` (exit 137); unsaved Rendered edits survive only via crash recovery. Froze in 10 sessions (B ×4 incl. T1-18b, C ×2, A ×3, D ×1), 7 of them dumped with gdb. All **7 gdb dumps** show the main thread in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ← ferrite::markdown::editor::arrow_nav_sel_key ← remember_arrow_nav_sel ← {render_session_plain_text_block \| render_session_formatted_edit_text ← render_paragraph \| closure} ← MarkdownEditor::show_rendered_editor`. No other thread waits on a parking_lot lock. **Source pointer (from reading, not verified by a fix):** `src/markdown/editor.rs:710-716` calls `ui.memory_mut(\|m\| m.data.insert_temp(arrow_nav_sel_key(ui), block))`, and `arrow_nav_sel_key` (`:696`) calls `ui.memory(…)`. A read lock on the egui context memory is requested while the same thread holds the write lock, so it self-deadlocks. Computing the key before `memory_mut` should fix it. Nothing in the stack is Linux-specific, so **check Windows too**. **Batch 5 (T2-35, ISS-31):** 2 more freezes with Preview lock ON (arrow keys in Rendered), both dumped with the same stack, so there are 9 dumps in total. **Batches 9–12 (Oct 9):** 10 more freezes, every one dumped with gdb and every dump showing the same `lock_shared_slow ← arrow_nav_sel_key ← remember_arrow_nav_sel ← … ← show_rendered_editor` frames: ARR-1 ×2 (ArrowDown after clicking a Rendered paragraph), ARR-2 ×2 (the ArrowDown that leaves a multi-line paragraph; Down inside the block works), ARR-4 ×2 (ArrowDown / ArrowUp with focus in the Split preview), CPY-3 ×1 (mouse drag over a paragraph that a double-click had put into edit view; caller `render_session_formatted_edit_text`), IND-9 ×1 (clicks + Tab on a list item in the Split preview; caller `render_session_plain_text_block ← render_list`), UX-9 ×2 (plain click on a file link inside a list item; caller `render_session_formatted_edit_text ← render_list`). Totals: **22 freezes, 19 gdb dumps, all the same stack.** | shots/T1-2-fail.png, T1-2-blocked-hang*.png, T1-7-fail-hang*.png, T1-17-fail.png, T1-17-retry-fail-hang.png, T1-18-fail-hang.png, T1-18b-fail-hang.png, T1-22-fail-hang.png; logs/T1-2-hang-bt.txt, T1-7-hang-bt.txt, T1-17-hang-bt.txt, T1-17-retry-hang-bt.txt, T1-18-hang-bt.txt, T1-18b-hang-bt.txt, T1-22-hang-bt.txt; debug logs HANG-A, HANG-B, 204753-batch2, 210253-batch2, 210656-batch2c, 210920-batch2d, 213134-T1-22 (+ log excerpts below); T2-35 evidence under ISS-31; Batches 9–12: shots/ARR-1-fail.png, ARR-1b-fail.png, ARR-2-fail.png, ARR-2b-fail.png, ARR-4-fail.png, ARR-4b-fail.png, CPY-3-fail.png, IND-9-fail.png, UX-9-fail.png, UX-9-fail-2.png; logs/ARR-1-hang-bt.txt, ARR-1b-hang-bt.txt, ARR-2-hang-bt.txt, ARR-2b-hang-bt.txt, ARR-4-hang-bt.txt, ARR-4b-hang-bt.txt, CPY-3-hang-bt.txt, IND-9-hang-bt.txt, UX-9-hang-bt.txt, UX-9-fresh-2-hang-bt.txt |
| ISS-2 | T2-36, VID-1, EXTRA-YT-2, EXTRA-YT-4, EXTRA-YT-6 (blocks VID-2/3/6-9/§9-§11, EXTRA-YT-1 A/C5-9/D, -3, -5, -8, EOL-7 resize); EXTRA-YT-7, VID-10 (Batch 12b) | **blocker** (crash) | **fixed (`97347d3`) — GrokBot re-run pending** | **Linux:** any trusted YouTube embed aborts the app: `GDK has not been initialized. Call gdk::init or gtk::init first.` | Linux X11 build (Windows uses WebView2 and is probably not affected): 1. Create a `.md` containing only `{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ}}` (or open `test_md/test_video_embeds.md`). 2. Switch to **Rendered** or **Split**. → abort. Also: in Split, type a trusted `{{video https://youtu.be/… width=320}}` line; the app aborts on the keystroke that completes it. Offline makes no difference. | Thumbnail plus play overlay; WebView on play, or degrade to a thumbnail if the WebView can't be created; never a crash. | Panic at `gdk-0.18.2/src/auto/display.rs:301:9` on the **first render** of the embed (before any click), exit 134 SIGABRT. Backtrace: `gdk::auto::display::Display::default ← ferrite::markdown::video_render::create_child_webview ← ferrite::markdown::editor::render_video_embed ← render_node ← … ← MarkdownEditor::show_rendered_editor`. `rg 'gtk::init\|gdk::init\|gtk_init' src/` finds nothing. wry's `build_as_child` needs GTK initialised on Linux. Untrusted hosts (vimeo, nocookie, evil-suffix, no-id, playlist-only) and invalid schemes correctly never create a WebView (VID-4, EXTRA-YT-1 B/C: PASS). Reproduced in 7 launches. **Batch 12b (EXTRA-YT-7):** a copy of `test_video_embeds.md` (embeds #1–#4) switched to Rendered aborted the same way in 2 more launches (05:01:39 and 05:02:39 CEST, exit 134, same backtrace), so **9 launches in total**. The overlay, dialog and Search-in-Files checks over playing players (EXTRA-YT-7) could not be reached. VID-10's "degrades to thumbnail; no crash" fails for the same reason. | shots/T2-36-fail.png, VID-1-fail-crash.png, EXTRA-YT-2-fail-offline-crash.png, EXTRA-YT-4-fail-crash.png, EXTRA-YT-6-fail-crash.png; debug logs 223058-T2-36, 223148-T2-36-retry, 223237-T2-36-split, 223520-VID-1, 223923-EXTRA-YT-4, 223957-EXTRA-YT-6, 224251-EXTRA-YT-2; shots/EXTRA-YT-7-fail.png; debug logs 20261009-045325-GIT-2, 20261009-050216-EXTRA-YT-7-fresh |
| ISS-3 | T1-4b (must-pass); RLD-7 (UTF-16 half, Batch 10) | **blocker** [major: file can't be opened] | **fixed (`5325e9b`) — GrokBot re-run pending** | UTF-16 LE file (with BOM) is classified as binary and handed to the OS default app | 1. Create a UTF-16 LE + BOM file with CRLF text (Notepad → Save As → UTF-16 LE; Linux: `printf '\xff\xfe' > f.md; iconv -f UTF-8 -t UTF-16LE crlf-test.md >> f.md`). 2. Open it in Ferrite (CLI argument or forwarded open). | Opens as text in a tab. Raw → Enter → Ctrl+S keeps `0D 00 0A 00` pairs. | Log: `Delegating to external app (binary): /tmp/crlf-utf16.md`. LibreOffice Writer opened it and no Ferrite tab appeared. `src/state.rs:278 is_binary_content()` returns true for **any `0x00` byte** (`bytes.contains(&0)`, line 285) before any BOM or encoding detection. Every UTF-16 file contains 0x00 bytes. **RLD-7 (Batch 10):** `/tmp/crlf-utf16.md` was again `Delegating to external app (binary)`, so UTF-16 reload/edit/save can't be tested in Ferrite. The Windows-1252 half of RLD-7 passes (`e9` kept). | shots/T1-4b-fail.png; logs/T1-4b-check.txt (fixture valid); debug log 202232-T1-4b; shots/RLD-7-fail-utf16.png; logs/RLD-7-utf16-check.txt |
| ISS-4 | T1-15 (must-pass); SIF-1; SIF-1, SIF-2, SIF-3 (Batch 7) | **blocker** [major] | **fixed (`5325e9b`) — GrokBot re-run pending** | Search in Files: clicking a CJK match opens the file with the caret at Ln 1, Col 1 instead of on the match | 1. Open the repo folder as workspace. 2. Create `test_md/zz_cjk_search.md` with `搜索测试 中文 search`. 3. Ctrl+Shift+F → `中文` → click the result for that file. | Caret (and highlight) **on** `中文` (Ln 1, Col 6). | No crash. The file opens, but the caret is at Ln 1, Col 1. The log shows the right target (`Opened file from search: … at line 1, char offset 5`, `Set transient highlight at 5..7`), and then the editor resets the cursor: `scroll_to_line 1 … cursor_before=Cursor{line:0,column:0}`, `after set_cursor, cursor_after=Cursor{line:0,column:0}`, `cursor sync changed: (0, 5) -> (0, 0)`. Reproduced twice. **Batch 7:** reproduced in SIF-1 (3×, `(0, 5) -> (0, 0)`), SIF-2 (`search`, `char offset 8`, `(0, 8) -> (0, 0)`) and SIF-3 (`istanbul` `(1, 18) -> (1, 0)`, `İstanbul` `(1, 9) -> (1, 0)`). The reset also happens for ASCII matches, so it is not CJK-specific; the computed offsets and highlights are correct. Only a match at column 1 ends up in the right place. | shots/T1-15-fail.png; debug log 204753-batch2 (lines quoted below); shots/SIF-1-fail.png, SIF-2-fail.png, SIF-3-click2-fail.png; debug logs 20261009-012958-SIF-1, 20261009-013635-SIF-2, 20261009-014003-SIF-3 |
| ISS-5 | T1-14; = IMG-3, IMG-9; IMG-X1, IMG-X2 (Batch 7); blocks IMG-4, IMG-7 (Batch 10) | **blocker** [major on Linux; Windows unknown] | **fixed (`97347d3`) — GrokBot re-run pending** | Ctrl+V with an image on the clipboard inserts nothing on X11; the context-menu Paste works | 1. Open a saved `.md`, caret at the end of `日本語テキスト ééé`. 2. Put a PNG on the clipboard (Snipping Tool; Linux: `xclip -selection clipboard -t image/png -i test_md/assets/sample.png`). 3. Press **Ctrl+V** (twice). 4. Then right-click › Paste. | Ctrl+V inserts `![](assets/…png)` at the caret, writes the PNG under `assets/`, and it renders. | Ctrl+V: nothing inserted, no PNG; log `ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` (×2). Right-click › Paste: inserted `![](assets/1791485625-clipboard.png)` and wrote `fixtures/assets/1791485625-clipboard.png` (6190 bytes). **Batches 7 and 10:** IMG-X1 (Untitled tab), IMG-X2 (saved doc; clipboard TARGETS = `image/png` only), IMG-4 and IMG-7 all hit the same arboard error: no PNG written, nothing inserted, no toast (ISS-46). Right-click › Paste was not retried in those rows. | shots/T1-14-fail.png; debug log 204753-batch2; shots/IMG-X2-fail.png; debug logs 20261009-020219-IMG-X1, 20261009-020435-IMG-X2, 20261009-034249-EMO-4 |
| ISS-6 | T1-19; = UND-1 | **blocker** [major] | **fixed (`51f5a3a`) — GrokBot re-run pending** | Split view: a preview code-block edit is undone one keystroke at a time instead of as one edit (#167) | 1. Open `test_md/test_copy_and_split_undo.md`, Split view. 2. In the preview, edit the Python code block in Part B: add a line `    # appended`, click Done. 3. Press Ctrl+Z once. | The whole edit reverts in one undo step. | Only the last char is removed (`# appende`), status "Undo (14 remaining)". The log shows 15 separate `Content modified in split rendered pane, recorded for undo` entries (19:16:08–09Z) for one edit. The first attempt, typing a newline in the preview, lost text and left blank lines. SENTINEL lines stayed intact. See also ISS-40 for the Rendered (non-split) variant found in UND-3. | No screenshot saved; debug log 211300-batch2e |
| ISS-7 | T1-21 (must-pass); = MMD-12 | **blocker** [major; no crash] | **fixed (`51f5a3a`) — GrokBot re-run pending** | Typing a backtick fence character by character: auto-close inserts a backtick after every keystroke and the text comes out reversed | 1. Ctrl+N, Split view. 2. Type character by character: ```` ```mermaid ````. 3. Continue with `flowchart TD`, etc. | Fence typed as written; banner may flicker; diagram renders when the fence closes; never crashes. | No crash (exit 0; malformed `A}x{ --> B` and `subgraph a] [b` don't crash either). But the opener became about 14 backticks followed by `diamrem` (mermaid reversed), and later lines landed at stale positions with more stray backticks. The log has **66** `` Auto-close: Inserted '`' after '`' `` lines, one after every keystroke: content grows by 2 while the caret advances by 1. Typing `flowchart TD` inside a pasted opener gave 11 auto-close inserts for 12 chars. Reproduced 3 times. After an Enter before the closing fence, the preview drew a stray ```` ``` ```` node. A pasted clean block renders correctly. **Caveat:** input was injected synthetically, so please confirm with a real keyboard; the log shows Ferrite's own auto-close doing the inserts. | shots/T1-21-fail.png, T1-21-malformed.png, T1-21-supp-clean-paste-diagram.png; debug log 212111-T1-20 |
| ISS-8 | T1-22; = MMD-8 (MMD-1 likely); MMD-1, MMD-11 (popup part, Batch 11) | **blocker** [major; hang part = ISS-1] | **fixed (`5325e9b`) — GrokBot re-run pending** | Mermaid #165: popup never opens; no Fit/Native toggle; no hover tooltip or hand cursor (Batch 11: popup does open from the expand icon; see Actual) | 1. Open `test_md/test_mermaid_issue_165.md`, Rendered. 2. Hover the widest flowchart (FC-165a). 3. Click the expand icon, the diagram body, and Ctrl+click it. | Hover shows the Fit/Native toggle plus the tooltip "Click diagram or Ctrl+Click to open popup" and a hand cursor; a click opens the zoom/pan popup. | Fit-to-pane OK, no gap below, no overpaint, wheel-scroll OK. But: no tooltip or hand cursor on hover; the header has only "Flowchart \| Source > mermaid" and the expand icon, and the source has no Fit/Native label; **6 clicks plus 1 Ctrl+click never opened the popup** (zoom/close untestable). Session 1 hung (ISS-1) when the expand icon was clicked while a paragraph was in edit mode. Session 2 (about 6 min of clicking without editing) did not freeze. **Batch 11 update (partly contradicts T1-22):** on `test_mermaid_v031.md` the expand icon (tooltip "Open diagram in popup") **did open** the popup; zoom in/out/reset, wheel-zoom in Hand mode and X all work, and a Fit/Native width toggle exists (MMD-9/MMD-11 used Native). Still failing: a plain click or Ctrl+click on the diagram body never opens the popup (4 tries, plus 1 on FC-83b, whose hover hint says "Click diagram or Ctrl+Click to open popup"); on FC-165a the expand icon did nothing in Fit mode (3 tries) but worked after switching to Native. Hand-pan could not be verified (synthetic drags never panned). | shots/T1-22-fail.png, T1-22-fail-hang.png, T1-22-probe.png, T1-22-hover-probe.png; logs/T1-22-hang-bt.txt; debug logs 213134-T1-22, 213643-T1-22b; shots/MMD-1-partial.png, MMD-1-hover-button.png, MMD-11-inline-fc165a.png; debug log 20261009-035014-MMD-1 |
| ISS-9 | EXTRA-SET-9 | **major** (settings data loss) | **fixed (`51f5a3a`) — GrokBot re-run pending** | A corrupt or wrong-typed `config.json` silently resets **all** settings to defaults and overwrites the file on exit | 1. Quit Ferrite. 2. Edit `config.json`: either replace it with `{ not json`, or set `"font_size": "abc"` in an otherwise good file. 3. Launch Ferrite, then quit cleanly. 4. Inspect `config.json`. | Bad field ignored or defaulted individually, or a `.bak` restore; the user is told (toast); the original file is not destroyed. | Only a log WARN (`contains invalid JSON: key must be a string at line 1 column 3` / `invalid type: string "abc", expected f32 … Using default.`). No toast, no `.bak` (none exists). **One bad field discards the whole config.** On exit `Configuration saved successfully`: config.json is rewritten with defaults, and the user's settings are gone for good. No crash or hang. | shots/EXTRA-SET-9a-pass.png; debug logs 221551-EXTRA-SET-9a, 221645-EXTRA-SET-9b |
| ISS-10 | EXTRA-SET-2-defect | **major** | **fixed (`5325e9b`) — GrokBot re-run pending** | Raw editor text invisible with Dark theme + Syntax Theme "ANSI" | 1. Settings: Theme Dark, Font JetBrains Mono, Font Size 20, Syntax Theme **ANSI**. 2. Open any `.md` in Raw. | Readable text. | Body text invisible; line numbers and minimap still drawn. Reproduced 2×. Switching to Base16 Mocha Dark makes the text visible again immediately. Light + ANSI not tested. | shots/EXTRA-SET-2-fail-ansi-invisible-text.png vs shots/EXTRA-SET-2-pass-mocha-visible.png; debug log 220315-EXTRA-SET-2-relaunch |
| ISS-11 | EXTRA-SET-4 (restart) | minor | **fixed (`5325e9b`) — GrokBot re-run pending** | "Recent Files: Remember last 0 files" is not honoured after restart | 1. Settings › Files › Recent Files slider → 0 ("Remember last 0 files"). 2. Quit cleanly, relaunch, open Settings › Files. | Slider still at 0. | config.json before relaunch had `max_recent_files=0`; after relaunch the slider reads "Remember last **10** files" (recent list empty, no crash). The other 4 Files settings persisted. | shots/set-persist-recent.png; debug log 222853-set-persist |
| ISS-12 | EXTRA-SET-1 (+ EXTRA-SET-3 note); TTL-6b (Ctrl+, part, see ISS-37) | minor | **closed — implemented** (round 1 task 22; Ctrl+, half = ISS-37 `5eec844`) — GrokBot re-run pending | Esc does not close the Settings tab; Ctrl+, did not open Settings | 1. Open Settings (ribbon gear). With no text field focused, press **Esc**. 2. With a document active, press **Ctrl+,**. | Esc closes Settings; Ctrl+, opens or focuses it. | Esc: no effect (1 attempt). Ctrl+,: no effect (several attempts across rows, and again in SI-4). **Possibly injection-related** (comma keysym with Ctrl through synthetic input), so re-check on a real keyboard. Also: clicking the gear while Settings is active closes it (toggle). **Batch 12b (TTL-6b):** `Keyboard shortcut: Ctrl+, (Open Settings)` was logged 3 times and Settings still did not open, so the Ctrl+, part is a real defect rather than input injection; see ISS-37 (`toggle_settings()` flips a flag nothing reads). Esc was not re-tested. | debug log 221729-EXTRA-SET-10 (no WARN for these actions); debug log 20261009-050403-TTL-6b-on |
| ISS-13 | T2-29 | — | **triaged — not a defect** (round 1 task 27; stale checklist; `iss-13-terminal-shell-settings-triage.md`) | No terminal-shell selector in Settings | Settings → Terminal chip, scroll to the bottom; search `shell`. | Row 29 lists "terminal shell" among the 0.3.0 controls. | Terminal shows Enable … Focus Terminal on Prompt, with no shell selector. Search `shell` → only "Code execution" and "Enable Integrated Terminal". The settings registry has no shell entry either. | shots/T2-29-pass-shell-search.png, T2-29-terminal-*.png; debug log 215656-set-extra |
| ISS-14 | T2-25b | minor | **closed — does not reproduce** (round 1 task 24; All ends at Essentials by design; ScrollArea height fix for overflow) | Settings "All" overview: wheel-scroll stops at Files › Auto-save delay; Recent Files, Keyboard and Terminal can't be reached in the All view | Settings, empty search, All → wheel-scroll to the bottom. | Every section reachable. | Scroll ends at "Auto-save delay". The missing sections are reachable through their chips. Needs a second look on a real display. | shots/T2-25b-pass.png; debug log 214507-T2-25-relaunch |
| ISS-15 | T2-23, T2-24, EXTRA-SET-10 | minor | **fixed (`51f5a3a`; `settings-keyboard-chip-search-count.md`) — GrokBot re-run pending** | Keyboard chip live count is wrong | Settings → query `zoom` (or `tab`). | Chip count = number of matching rows. | Keyboard chip shows (1) while 3 (`zoom`) or 4–5 (`tab`) rows show. | shots/T2-23-pass-zoom.png, T2-24-pass.png |
| ISS-16 | T2-25 | minor (doc/code mismatch) | **closed — checklist aligned to code** (`51f5a3a`; `settings-recently-changed-list.md`) | "Recently changed": shortcut rebinds not listed; checklist cap 6 vs code cap 8 | Change theme, font size and a shortcut → reopen Settings. | Row 25: all three listed, max 6. | Only Theme and Font Size listed; the rebind (Toggle Word Wrap → Ctrl+Alt+K) is not (code excludes rebinds by design). Code: `MAX_RECENTLY_CHANGED = 8` (`src/ui/settings/registry.rs:18`); the GUI showed at most 2 entries, so the cap wasn't exercised. *Accent:* after EXTRA-SET-2 the block **did** list "Ferrite accent", so accent is not missing in this build. | shots/T2-25-pass.png |
| ISS-17 | T2-28; EXTRA-SET-14 (Batch 12c) | minor | **fixed (`5325e9b`) — GrokBot re-run pending** | Settings › Reset All has no confirmation | Settings → Reset All. | Confirm dialog (checklist: "Reset All → confirm"). | Resets immediately (status "Settings reset to defaults"). Side effect seen later: it re-enabled "Restore previous session". **Batch 12c (EXTRA-SET-14):** confirmed again: one click resets everything, no confirm step (code: the button sets `reset_requested` directly, src/ui/settings/mod.rs ~L225). Defaults survive a restart. Also: the Recently Changed list is not cleared in-session (its 8 entries stay, now showing default values); it is cleared on quit (`recently_changed_settings: []`). | shots/T2-28-pass.png; shots/EXTRA-SET-14-defaults.png; debug logs 20261009-053015-EXTRA-SET-13-restart, 20261009-053158-EXTRA-SET-14-relaunch |
| ISS-18 | T2-23, T2-27, EXTRA-SET-10 | minor (possibly input-injection) | **closed — implemented** (round 1 task 23; `settings-search-initial-focus.md`) — real-keyboard check pending | First keystroke(s) dropped in Settings search and in the shortcut-capture popup | Type `bold` into Settings search right after Ctrl+A; open a chord-capture popup and press a chord once. | Every key lands. | `bold` → `ld`; the first chord press was swallowed (the second worked); `[` dropped as the first char. Synthetic input may contribute; re-check on a real keyboard. | debug logs 213643-T1-22b, 214507-T2-25-relaunch, 221729-EXTRA-SET-10 |
| ISS-19 | EXTRA-SET-8 | minor | **fixed (`51f5a3a`; `terminal-settings-validation.md`) — GrokBot re-run pending** | No validation or feedback for Terminal startup command, Custom Prompt Patterns (invalid regex), Sound file path | Enter `([` in Custom Prompt Patterns; `/nonexistent.wav` as sound file; `;;;` as startup command. | Invalid values rejected or flagged. | Accepted silently, no indicator (no WARN in the log). Max Line Width, by contrast, clamps correctly (0 → 400, -5 → 400, 99999 → 2000, `abc` ignored). | debug log 220823-EXTRA-SET-3-relaunch |
| ISS-20 | T1-11 | polish | **fixed (`5325e9b`) — GrokBot re-run pending** | Session Recovery dialog shows the raw i18n key `recovery.session.tabs_unsaved` | Edit a saved tab, kill the process, relaunch. | Translated sentence. | Literal `recovery.session.tabs_unsaved` between the dialog sentences. | shots/T1-11-pass.png |
| ISS-21 | T1-8 → T1-10 (observation) | minor | **fixed (`51f5a3a`) — GrokBot re-run pending** | A tab closed during the session reappears in the next session-restore/recovery tab list | Open A and B; close B; kill and relaunch. | B not restored. | `other-tab.md` (closed in T1-8) is listed in "Restoring N tab(s)" and in the tab bar of the recovery state. It disappears after Restore Session. Disk untouched. | debug logs 203037-T1-10, 203215-T1-11 |
| ISS-22 | T1-11 | minor | **fixed (`51f5a3a`) — GrokBot re-run pending** | "Recovered content differs from this file on disk" banner stays after Ctrl+Z makes the buffer equal to disk | Kill → Restore Session → Ctrl+Z once. | Banner disappears. | Banner remains. | shots/T1-11-pass.png; debug log 203534-T1-11-dialog |
| ISS-23 | T1-11 | minor | **fixed (`51f5a3a`) — GrokBot re-run pending** | Stale recovery file left after a clean exit | After the T1-11 restore + undo, Alt+F4 (no dialog, exit 0). | `recovery/` cleaned. | `~/.config/ferrite/recovery/12.json` left behind containing the undone edit `KILL-EDIT` (T1-10 left the dir empty). | debug log 203534-T1-11-dialog |
| ISS-24 | T1-16, EXTRA-SET-3, EXTRA-SET-8, batch 4 | minor (environmental; mitigated) | **closed — mitigated** (round 1 task 25; Xvfb/llvmpipe-only in QA; `request_repaint` safeguard) | Repaint lag: UI updates only after the pointer moves | Find results, checkbox toggles, typed text in Settings fields, Rendered/Split pane. | Immediate repaint. | Stale frame until a mouse move ("No matches" shown until the pointer moved; checkbox looked unchanged). Seen only on Xvfb + llvmpipe. | shots/T1-16-pass.png |
| ISS-25 | T1-13, EXTRA-SET-3 | polish | **fixed (`51f5a3a`; `status-bar.md`) — GrokBot re-run pending** | Stale status-bar info | Duplicate a line (Ctrl+Shift+D); turn Vim mode off with the Settings tab active. | Status bar updates. | Line count stayed 10 while 11 lines showed; the Settings-tab status bar kept `[NORMAL]` with Vim off. | debug log 210253-batch2 |
| ISS-26 | VID-5 | minor | **fixed (`51f5a3a`; `autolink-rendered-styling.md`) — GrokBot re-run pending** | Bare URLs and `<https://…>` autolinks are not link-styled in Rendered | Rendered doc with a bare `https://…` and `<https://…>`. | Rendered as links. | Plain black text (`[text](url)` is blue). A control doc without video behaves the same. | shots/VID-4-pass.png; debug log 223549-CTRL-plain |
| ISS-27 | T2-29 | minor | **fixed (`5325e9b`) — GrokBot re-run pending** | Open Workspace and Close Workspace are both bound to F12 | Settings › Keyboard. | Unique default bindings. | Both rows show F12. | shots/T2-29-*.png |
| ISS-28 | Automated gate | minor (test hygiene) | **fixed (`51f5a3a`; `temp-data-dir-test-helper.md` audit test) — verify on next Linux `cargo test`** | `cargo test` writes to the real user config dir | Run `cargo test` on a machine with no `~/.config/ferrite`. | Tests use a temp dir. | `~/.config/ferrite/config.json` and `recovery/` created (prep observation, gui-test-plan §1; the dir existed before the first GUI launch and was moved aside). | logs/cargo-test.log, results-batch1.md Batch 0 |
| ISS-29 | EXTRA-SET-10 | polish | **closed — implemented** (round 1 task 26; `settings-search-emoji-font.md`) | Emoji 🙂 accepted but invisible in the Settings search field | Paste 🙂 into Settings search. | Glyph visible (Noto Color Emoji installed). | Blank field; every launch logs WARN `epaint … Failed to find replacement characters`. | shots/EXTRA-SET-10-pass.png |
| ISS-30 | T2-32; = EMO-1, EMO-2 (related: ISS-29) | major | **documented limitation (task 25)** — monochrome/blank emoji expected; re-run T2-32 for regression only | Emoji are drawn monochrome and some common emoji are not drawn at all, although Noto Color Emoji is installed and registered | 1. Linux: check that `fc-list \| grep -i emoji` lists `/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf` (Windows: Segoe UI Emoji is present by default). 2. Open `test_md/test_emoji.md` in **Raw**. 3. Look at line 9 (`## Common single-codepoint emoji`: 😀 😅 🤔 😴 🥳 😢 😡) and at the symbols, skin-tone, ZWJ and flag lines. 4. Switch to **Rendered** and compare. | Every emoji visible, in colour since a colour emoji font is installed; no blanks or tofu; UI icons unchanged. | Raw: only 5 of the 7 common emoji are drawn, and the last two positions are blank (no glyph, no tofu box). All glyphs are monochrome. Rendered: the same line has 5 glyphs, with blanks at positions 4 and 7 (identical after about 1 min). The variation selector after ⚠ shows as an empty box; ZWJ sequences split into component glyphs with one empty box; flags show as letters `N O U S J P D E`. Toolbar and tab icons are fine. Log: "Found system emoji font at … NotoColorEmoji.ttf", "Registered system emoji font fallback", "Pre-warmed font atlas with … 9 emoji characters"; every launch also logs WARN epaint "Failed to find replacement characters". No crash, exit 0. | shots/T2-32-raw.png, T2-32-rendered.png, T2-32-rendered-recheck.png; logs/T2-32-fail-tail.txt; debug log 230020-T2-32 |
| ISS-31 | T2-35; = ARR-3 (same root cause as ISS-1) | **blocker** (hang; known ISS-1 deadlock) | **fixed (`5325e9b`) — GrokBot re-run pending** (same root as ISS-1) | Preview lock ON: arrow keys don't navigate the Rendered view, and after a click into the pane one ArrowDown freezes the app | 1. Open `test_md/test_find_arrow_nav.md`. 2. Turn **Preview lock** ON (padlock). 3. Switch to **Rendered**. 4. Press ArrowDown 3 times. 5. Click a blank area of the Rendered pane (not on text) and press ArrowDown once. | Arrows move/scroll the Rendered view; edits stay blocked; the app stays responsive. | Step 4: no visible scroll (before/after screenshots pixel-identical). Step 5: the UI froze, the log stopped advancing, the process stayed alive; kill -9 (exit 137). gdb: main thread in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ← ferrite::markdown::editor::arrow_nav_sel_key ← remember_arrow_nav_sel ← … ← MarkdownEditor::show_rendered_editor`, the same stack as ISS-1. Froze in 2 launches (the first on arrow keys in Rendered, the second after the click + 1 Down). The lock itself works: Rendered typing is blocked, the Raw/Split-left pane stays editable, and the lock persists across restart. The ISS-1 fix should cover this. | shots/T2-35-fail.png, T2-35-rendered-lock-on.png, T2-35-rendered-lock-typed-Q.png, T2-35-split-raw-typed-Z.png; logs/T2-35-hang-bt.txt, logs/T2-35-relaunch-hang-bt.txt; debug logs 230821-T2-35, 231259-T2-35-relaunch-a |
| ISS-32 | T3-43; = MMD-7; MMD-11 (frame overlap), MMD-4 (side note) (Batch 11) | major | **triaged — defer 0.3.2** (task 15; `prd-v0.3.1-mermaid-fidelity.md` §12) | Mermaid flowcharts with subgraphs: edges cross subgraph titles, nested subgraphs overlap or are laid out flat, and one diagram drops a node, an edge and a label | 1. Open `test_md/test_flowcharts.md`, **Rendered**. 2. Look at "Subgraph Test (TD)", "Edge Routing Across Subgraph Boundaries (TD)", "Nested with Direction Override" and "Asymmetric Shape with Dash-Style Edge Labels". 3. Then "Nested Subgraph Test (Sibling Subgraphs)", "True Nested Subgraphs" and "Deeply Nested Subgraphs (3 Levels)". | Edges avoid nodes and titles (FC-83a regression); each layout matches the block's Expected text. | (1) Subgraph Test (TD): the Node 2 → Node 3 arrow runs through the "Inner Group" title. (2) Edge Routing (TD): Start → Step 1 crosses the "Process A" title, and Step 2 → Step 3 crosses "Process B". (3) Nested with Direction Override: the Input Stage box overlaps the Main Process container and hides its title; Output Stage sits outside Main Process. (4) Asymmetric Shape with Dash-Style Edge Labels: the "Two line / edge comment" label is clipped behind Result One and hides the Odd → Result One arrow; "Simple label" on Odd → Result Two is not drawn; node "Flag" (A) and its dotted edge to Target are not drawn. Also: Sibling Subgraphs draws `one --> two` as two extra boxes "one"/"two" instead of linking the subgraphs and ignores `direction LR`; True Nested and Deeply Nested show the subgraphs side by side, not nested. No error boxes, no crash. Many other diagrams are clean (list in `results-batch6.md`). **Batch 11:** FC-165a: the 统一接入层 subgraph frame starts inside the 业务客户端 PEP frame and hides its right border (MMD-11; no node overlap, fan-outs and `<-->` arrowheads correct). `test_mermaid_v031.md` §4: subgraph `Short` overlaps the S1 box, its title is not drawn, and S1/S2 render as plain nodes inside S1 (side note in MMD-4, whose width criterion passes). | shots/T3-43-fail-subgraph-test-td-title-overlap.png, T3-43-fail-edge-routing-td-title-overlap.png, T3-43-fail-nested-direction-override-overlap.png, T3-43-fail-dash-edge-label-overlap.png, T3-43-flowchart-subgraph-td.png, T3-43-flowchart-complex-cross.png; debug log 234101-T3-43; shots/MMD-11-popup-native-100pct.png, MMD-4-pass.png |
| ISS-33 | T3-44; = HTML-1; HTML-6 (Batch 12a) | major | **fixed (task 14) — GrokBot re-run T3-44/HTML-6 pending** | Two adjacent single-line `<div align>` blocks are not rendered; the document's first line is drawn in their place | 1. Open `test_md/test_github_html.md`, **Rendered**. 2. Scroll to "Adjacent divs", which contains `<div align="center">First centered block.</div>`, a blank line, and `<div align="right">Second block, right-aligned.</div>`. | "First centered block." centred and "Second block, right-aligned." flush right. The repo docs (`docs/technical/markdown/github-html-block-subset.md`, test `test_html_single_line_div_align`) say this single-line form is supported. | Neither text appears. The document's first line, `A# GitHub HTML Subset Test` (that is the literal first line of the committed fixture), is drawn twice in their place, once indented. Multi-line `<div align>`, `<details>` and `<kbd>` render correctly. No crash. Side note: `<strong>` inside a `<summary>` shows as small grey tag chips, and the summary text is not bold. **HTML-6 (Batch 12a)** reproduces it: both single-line divs show `A# GitHub HTML Subset Test`, left-aligned; the following paragraph is fine. The single-line `<div onclick>` in the Unsafe section shows only an «HTML» indicator (acceptable per the fixture). | shots/T3-44-fail-adjacent-single-line-divs.png, T3-44-adjacent-divs-check.png; debug log 234543-T3-44; shots/HTML-6-fail.png; logs/batch12/HTML-6-tail.txt |
| ISS-34 | SI-1; = MW-6 (partly) | minor (possibly WM-related) | **fixed (task 26)** — owning-viewport focus; xfwm4 may still block raise (note BLOCKED if WM-only) | A forwarded open selects the right tab but doesn't raise its window when another Ferrite window is on top | Linux X11 (xfwm4): 1. `ferrite /tmp/lf-test.md`. 2. Have a second Ferrite window (e.g. Ctrl+Shift+N, or a restored 2-window session) and bring it to the front. 3. From a terminal: `time ferrite /tmp/lf-test.md`. Windows: same with two Ferrite windows and Explorer → Open. | The window holding lf-test.md is raised and focused (#147); the 2nd process exits in ≲1 s. | The 2nd process exits in 0.095 s (rc 0), and the primary logs "File already open, switching to window 0 tab 3", but the active X window stays the other Ferrite window. The log shows only `winit::Window::request_user_attention; request_type=Some(Informational)` ×2, with no focus or raise request. Reproduced (0.080 s). In T2-38, with a single Ferrite window, the window **was** raised. This may be xfwm4 focus-stealing prevention turning the raise into an attention hint, so check on Windows and another WM before fixing. | shots/SI-1-fail.png; debug log 235629-SI-1 |
| ISS-35 | T3-41 (PARTIAL, supplementary fixture); TBL-4 | minor | **fixed (`5eec844`) — GrokBot re-run T3-41 supp pending** | Bold inside a link in a table cell shows literal `**` | 1. Open `test_md/task_50_table_inline_formatting.md`, **Rendered**. 2. Find the cell `[**bold link**](https://example.com)`. | Bold, underlined link text "bold link". | Underlined link text with the `**` asterisks visible. Other inline formatting in cells (bold, italic, strikethrough, bold+italic, code, strike+code, plain links) renders styled. Found on a supplementary fixture, so it is not a row FAIL. | shots/T3-41-supp-inline-format-task50.png; debug log 233908-T3-41 |
| ISS-36 | SES-6 | **blocker** (possible data loss; seen once, needs confirmation) | **hardened (tasks 1–4; diagnostic `5eec844`) — GrokBot re-run SES-6 pending** | After a session restore, typing into a restored file tab replaced the whole buffer with the typed text | 1. Open a saved multi-line file (SES-6 used `/tmp/ses1-a.md`, a copy of `lf-test.md`, saved in SES-1, restored by the session) and quit cleanly so the session restores it. 2. Relaunch; the tab comes back. 3. Click into that tab, press End, type ` W1`. 4. Check the editor text and the status bar. | ` W1` is appended at the caret; the rest of the document stays. | The editor showed only ` W1` (status "1 words, 4 chars, 2 lines"), and the recovery snapshot for that tab also held only ` W1`. The restored text was gone from the buffer. The file on disk was unchanged because the tester chose Don't Save at exit; saving was not tried, but Save would presumably have written the ` W1` buffer to disk. Seen once (SES-6), cause not established, no panic, exit 0. | shots/SES-6-window1-dialog.png; logs/ferrite-debug-20261009-024942-SES-6.log |
| ISS-37 | TTL-6b; explains ISS-12 (Ctrl+,) | major | **fixed (`5eec844`) — GrokBot re-run TTL-6b pending** | With *Use system title bar* ON there is no way to open Settings: the gear exists only in the custom title bar and Ctrl+, does nothing | 1. Settings › Appearance › tick **Use system title bar**, restart Ferrite. 2. Look for the Settings gear. 3. Press **Ctrl+,** (any window, Windows too: the shortcut part is platform-independent). 4. Try Ctrl+Shift+P. | Settings opens (Ctrl+, or another entry point), so the option can be switched back off in the UI. | The native title bar appears (the setting itself works), but no gear is drawn. The log records `Keyboard shortcut: Ctrl+, (Open Settings)` at 03:04:52, 03:05:32 and 03:06:01Z, yet no Settings tab opens; Ctrl+Shift+P showed no palette. The only way back was editing `config.json` (`"use_system_title_bar": false`) with Ferrite closed. Source pointer (from reading, not verified by a fix): `toggle_settings()` only flips `ui.show_settings`, and `rg show_settings src` finds nothing that reads it. Because the shortcut reached Ferrite and was logged here, the Ctrl+, half of ISS-12 is probably **not** an input-injection artifact. | shots/TTL-6b-on-native-titlebar.png, shots/TTL-6b-pass.png; logs/ferrite-debug-20261009-050403-TTL-6b-on.log, logs/ferrite-debug-20261009-050735-TTL-6b-off.log |
| ISS-38 | SES-1 (related: SES-5, T3-46 observation) | major | **by design + dialog (`5eec844`, task 28 doc) — GrokBot re-run SES-1/5 pending** | Exit → Save saves the file tabs but silently skips Untitled tabs (no Save As) | 1. Open two saved files and one new Untitled tab (Ctrl+N); type into all three. 2. Close the app (Alt+F4) → "You have unsaved changes. Exit anyway?" → **Save**. | Every file tab is saved, a Save As dialog appears for the Untitled tab, then the app exits. | Both files were saved (disk verified), then the app exited (code 0) with no Save As. The Untitled text went only to session recovery (`Saved recovery content for tab 69 (path=None, hash=None)`); `fixtures/untitled-save.md` was not created. The next launch restores it (SES-5), so it is not lost, but the user's Save choice is not honoured for that tab. With only Untitled tabs dirty, Alt+F4 quits with no prompt at all (SES-5, T3-46). | shots/SES-1-fail-dialog.png, shots/SES-1-fail.png; logs/ferrite-debug-20261009-024031-SES-1.log |
| ISS-39 | CPY-2 | major | **fixed (task 12) — GrokBot re-run CPY-2 pending** | Copying a mouse selection from a syntax-highlighted code block in Rendered inserts spaces at token boundaries (#162) | 1. Open `test_md/test_copy_and_split_undo.md`, Rendered. 2. In Part A, select part of the rust line `let x: Vec<String> = items.iter().map(\|s\| s.to_string()).collect();` with the mouse. 3. Ctrl+C and paste into a plain-text editor. | Byte-identical to the source. | `tems.iter().m` was copied as `tems. iter(). m`; a longer selection as `let x: Vec< String> = items. iter(). map(\| s\| s.` (spaces after `.`, `)`, `<`, `\|`). The block's **Copy** button gives the exact line (`cmp` identical), so only the selection path is affected. Caveat: only partial selections could be made with synthetic mouse drags. | shots/CPY-2-fail.png; logs/CPY-2-selection-clipboard.txt, logs/ferrite-debug-20261009-031127-CPY-2.log |
| ISS-40 | UND-3 (Split-view variant: ISS-6) | major | **fixed (tasks 8–9) — GrokBot re-run UND-3 pending** | Rendered (non-split) code-block editing: typed characters are dropped or misplaced, and undo goes back one keystroke at a time | 1. Open `test_md/test_copy_and_split_undo.md`, Rendered (not Split). 2. Part B Python block → **Edit** → Ctrl+End → Enter → type `alpha beta gamma` → **Done**. 3. Press Ctrl+Z once. | Typed text lands as typed; one Ctrl+Z removes exactly the appended line. | Pass 1: only the `U` of `UND3 added three words` landed and the block left edit mode. Pass 2: the text came out garbled (`return valuea b` / `eta gamma`). After Done, each Ctrl+Z removed only one character; the log shows `Undo performed, 12 entries remaining`, then `11 entries remaining`, i.e. one undo entry per keystroke. SENTINEL lines intact; not saved. Synthetic typing may contribute to the dropped characters; the per-keystroke undo is visible in the log. | shots/UND-3-fail.png; logs/ferrite-debug-20261009-031831-UND-3.log |
| ISS-41 | TBL-2; related EOL-3, EOL-5, ARR-6b, T3-41 observations | major | **fixed (tasks 5–7) — GrokBot re-run TBL-2 pending** | Rendered/preview edits reach the document late: a table-cell edit is missing from a save made before the cell loses focus, and the Raw pane, status bar and dirty marker stay stale | 1. Open a copy of `test_md/test_tables.md`, Rendered. 2. Click into a table cell (e.g. "More") and type `X`. 3. Without clicking another cell, switch to Raw and press Ctrl+S. 4. Inspect the file on disk. | The `X` is in Raw and in the saved file; the tab shows `*` as soon as you type. | Raw showed `More` and the saved file had `More`; the `X` reached Raw and disk only after going back to Rendered and clicking another cell. In EOL-3/EOL-5/ARR-6b a preview or Rendered paragraph edit did not update the Raw pane, status bar or dirty marker until Ctrl+S or a switch to Raw (those saves did contain the edit); T3-41 showed stale Raw text after a cell edit. Whether a pending cell edit is lost on save + quit was not tested. | shots/TBL-2-rendered-before.png, shots/TBL-2-after-edit.png, shots/TBL-2-raw-after.png, shots/TBL-2-pass.png, shots/EOL-3-pass.png; logs/ferrite-debug-20261009-041820-WS-1.log, logs/ferrite-debug-20261009-023554-EOL-3.log |
| ISS-42 | UX-9 | major | **fixed (task 13) — GrokBot re-run UX-9 pending** | Local links in Rendered: Ctrl+click / middle-click on file, folder and web links do nothing (#157); a plain click enters edit mode and then deadlocks (ISS-1) | 1. Open `test_md/test_local_links.md`, Rendered. 2. Ctrl+click a file link, a folder link and a web link. 3. Middle-click each. | File links reveal in the file manager, folders open, web links open in the browser. | Nothing opened: no thunar / xdg-open / browser process and no log line. A plain click on a file link switched the list item to source and froze the app (ISS-1, 2 gdb dumps). Ignored schemes not tested. The file manager itself works on this box (T2-39 Reveal opened thunar). | shots/UX-9-fail.png, shots/UX-9-fail-2.png; logs/ferrite-debug-20261009-041820-WS-1.log, logs/ferrite-debug-20261009-044244-UX-9-fresh.log |
| ISS-43 | EXTRA-SET-5 | major | **fixed (`5eec844`) — GrokBot re-run EXTRA-SET-5 pending** | Terminal *Auto-load Layout* OFF is ignored: opening the terminal panel still loads the workspace layout | 1. Open a workspace that has `.ferrite/terminal-layout.json`. 2. Settings › Terminal › **Auto-load Layout** OFF; restart. 3. Open the terminal panel (ribbon or Ctrl+`). | The saved workspace layout is not loaded. | The setting persisted (`terminal_auto_load_layout: false`), but the first panel open logged `Loaded workspace layout from "…/.ferrite/terminal-layout.json"` and `Terminal panel initialized from workspace layout` (03:13:18Z). Source pointer (from reading): `TerminalPanelState::toggle()` calls `initialize()` (src/ui/terminal_panel.rs ~L152), which calls `try_load_workspace_layout()` (~L344) without checking the flag; the only check is in src/app/navigation.rs:431. The other 9 terminal settings persisted and took effect. | shots/EXTRA-SET-5-fail.png; logs/ferrite-debug-20261009-050949-EXTRA-SET-5.log, logs/ferrite-debug-20261009-051246-EXTRA-SET-5-relaunch.log |
| ISS-44 | SIF-3 | minor | **fixed (task 19) — GrokBot re-run SIF-3 pending** | Case-insensitive search does not match `İstanbul` (U+0130) for the query `istanbul` | 1. A file containing `Istanbul İstanbul istanbul`. 2. Ctrl+Shift+F, case-insensitive, query `istanbul`. | All three variants found (brief §6 SIF-3). | 2 hits (`Istanbul`, `istanbul`); `İstanbul` is found only with the exact query. Offsets after `İ` are correct (highlights 24..32 and 33..41), so there is no byte/char drift. Clicking still resets the caret (ISS-4). Note: plain lower-casing maps `İ` to two code points (`i̇`), which may be the cause; whether `İ` should match `i` is a product decision. | shots/SIF-3-query.png, shots/SIF-3-Idotted-query.png, shots/SIF-3-click-Idotted-status.png; logs/ferrite-debug-20261009-014003-SIF-3.log |
| ISS-45 | PRI-3 | minor | **fixed (`5eec844`) — GrokBot re-run PRI-3 pending** | Middle-click on a tab does not close it (Linux; Middle-click paste ON) | 1. Open several tabs. 2. Middle-click one tab (on Windows: same gesture). | The tab closes. | 4 tabs tried (one twice, after hovering): none closed, no tab-close line in the log. Middle-click paste in the editor works (PRI-2). Not tried with Middle-click paste OFF or on Windows. | shots/PRI-3-fail-tab-still-open.png; logs/ferrite-debug-20261009-015853-PRI-1.log |
| ISS-46 | IMG-X1, IMG-X2 (+ T1-14, IMG-4, IMG-7) | minor | **fixed (`5eec844`) — GrokBot re-run IMG-X1/X2 pending** | A failed image paste is silent: no toast when the clipboard read fails | Linux X11: `xclip -selection clipboard -t image/png -i test_md/assets/sample.png`, then Ctrl+V in a saved doc and in an Untitled tab. | Image inserted, or a toast that says why not (for Untitled: "save the document first"). | Nothing on screen. Only `ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` in the log. Why the paste fails is ISS-5; this issue is only the missing user feedback. | shots/IMG-X2-fail.png; logs/ferrite-debug-20261009-020219-IMG-X1.log, logs/ferrite-debug-20261009-020435-IMG-X2.log |
| ISS-47 | SPL-2, SPL-5 | minor | **fixed (task 20) — GrokBot re-run SPL-2/5 pending** | Spellcheck misses `mistaek` and flags the link text `ok` | 1. Settings › Editor › Spellcheck ON. 2. Raw: `This is a speling mistaek in Ferrite.` 3. Raw: `[ok](http://speling.com)`. | `speling` and `mistaek` flagged; nothing inside link syntax flagged (brief SPL-2, SPL-5). | `speling` flagged; `mistaek` not flagged (it was not in `user-words.txt` at that time). `ok` in the link text is squiggled (the destination is not). Both may come from the dictionary/heuristics rather than tokenisation; needs triage. | shots/SPL-2-fail.png, shots/SPL-5-fail.png; logs/ferrite-debug-20261009-020610-SPL-1.log |
| ISS-48 | SPL-7 | minor | **fixed (task 21) — GrokBot re-run SPL-7 pending** | Spellcheck squiggles are drawn away from the misspelled word, at the right edge of the text column | 1. Spellcheck ON. 2. Open `test_md/test_scroll_perf.md` in Raw and scroll. | Each squiggle sits under its word. | Squiggles appear at the far right edge of the text column, about one wrap-width away from the word (e.g. `viewport`). Squiggles are limited to the viewport and scrolling stays smooth. | shots/SPL-7-scroll1.png, shots/SPL-7-scroll2.png; logs/ferrite-debug-20261009-020610-SPL-1.log |
| ISS-49 | STAT-3 | minor (needs triage: may be a checklist expectation) | **triaged — closed (task 18; by design)** | Raw editor: Enter after a list item does not continue the list | New doc, Raw: type `- item` Enter, `* item` Enter, `1. item` Enter. | The new line starts with `- `, `* `, `2. `. | No marker; caret at Col 1 each time. Stats correct, no panic. `split_list_item` (src/markdown/ast_ops.rs) is only called from the Rendered editor. If list continuation in Raw is not a feature, close this as a checklist issue. | shots/STAT-3-fail.png; logs/ferrite-debug-20261009-023201-STAT-3.log |
| ISS-50 | EOL-9 (optional row) | minor | **triaged — closed (task 18; by design)** | A mixed-EOL file keeps its per-line endings on save instead of one majority ending | 1. `printf '# Mixed\r\n\r\nline crlf\r\nline lf\nend\r\n' > mixed-eol.md` (4 CRLF, 1 LF). 2. Open, append ` x` to `end`, Ctrl+S. 3. `file mixed-eol.md`, `cat -A`. | All lines CRLF. | Still "with CRLF, LF line terminators"; `line lf$` kept its LF; the edited line was saved with CRLF. No crash. | shots/EOL-9-fail.png; logs/EOL-9-check-pre.txt, logs/EOL-9-check.txt, logs/ferrite-debug-20261009-023851-EOL-9.log |
| ISS-51 | IND-2 | minor | **fixed (task 22) — GrokBot re-run IND-2 pending** | Shift+Tab does not outdent a line indented with a hard tab | 1. `test_md/test_indent_selection.md`, Raw. 2. Select Block 2 lines 13–15 (4 spaces, 8 spaces, one tab) → Shift+Tab. 3. Caret only on the tab line → Shift+Tab. | Every line outdents one step, the tab line included. | Space-indented lines outdent one step; the `\t` line is unchanged in both cases (disk-verified with `cat -A`). Once (not reproduced later) a caret-only Shift+Tab on line 13 outdented line 14 instead. | shots/IND-2-fail.png; logs/ferrite-debug-20261009-032305-RLD-3.log |
| ISS-52 | UX-8 | minor | **fixed (tasks 10–11) — GrokBot re-run UX-8 pending** | Click-to-edit puts the caret about 10 source characters too far right on mixed-format lines | 1. Rendered line with bold, italic, code and a link followed by plain text ending in `tail`. 2. Click on `tail`. | Caret at the clicked position. | Caret about 10 source characters to the right ("after 'text '" per the tester's note). Plain text and line-start clicks are accurate. Reproduced on two scratch copies. | shots/UX-8-clicked.png, shots/UX-8-fail.png; logs/ferrite-debug-20261009-041820-WS-1.log |
| ISS-53 | UX-11 | minor | **fixed (`5eec844`) — GrokBot re-run UX-11 pending** | The Markdown quick reference (ribbon "MD") does not list the new Reload and word-wrap shortcuts | Click the ribbon **MD** button ("Markdown quick reference") and scroll to the end. | Ctrl+Shift+R (Reload) and Alt+Z (word wrap) listed (row UX-11). | Only formatting entries (Bold … Task list). F1 help does list both. Possibly a checklist expectation, since the popup is a Markdown syntax guide. | shots/UX-11-partial.png; logs/ferrite-debug-20261009-041820-WS-1.log |
| ISS-54 | TTL-6a (+ Batch 4/5b observations) | minor (title-bar rows are minor per brief §5) | **fixed (task 27) — GrokBot re-run TTL-6a pending (real X11 display)** | Custom title bar on Linux: double-click restore leaves the window full-size and partly off-screen; dragging from maximized does not restore | Linux X11 (xfwm4), default custom title bar: 1. Double-click the title bar (maximize), double-click again (restore). 2. Maximize, then drag the title bar downwards. | Restore returns to the previous size and position; drag-from-maximized restores and follows the pointer. | After the restore double-click the icon flipped but the window stayed 1920×1200 at x=−232 (file tree off-screen); another double-click did not maximize. The maximize button works and a restored window drags fine (+120/+85 px). Two drags from maximized left it maximized at 0,0. Edge-resize not achieved (the drag hit another window). No geometry lines in the log. Synthetic slow drags on xfwm4 without a compositor. Earlier batches saw a first title-bar click move the window ~350 px and a top-right button toggle Zen Mode once. | shots/TTL-6-a-dblclick-restore.png; logs/ferrite-debug-20261009-045325-GIT-2.log |
| ISS-55 | RUN-1 | minor | **fixed (`5eec844`) — GrokBot re-run RUN-1 pending** | Code run: output printed before Stop or a timeout is lost (Python stdout block-buffered) | 1. `test_md/test_code_execution.md`, Rendered, code execution enabled. 2. Run §11 (loop printing ticks), click **Stop** after ~10 s. 3. Run §10 (prints, then `time.sleep(60)`), wait for the timeout. | Output printed before Stop/timeout is shown (fixture §11 expects `Running…` and ticks). | "Stopped by user" / "Timed out after 30s" with "(no stdout or stderr)". Source pointer (from reading): `code_execution.rs` sets no `PYTHONUNBUFFERED`, so piped Python output stays in the buffer and dies with the process. Consent, normal output, timeout and Stop themselves work. The default 30 s timeout applied; the fixture's 5 s was not applied (not investigated). | shots/RUN-1-consent-pass.png; logs/ferrite-debug-20261009-044740-RUN-1.log |
| ISS-56 | EXTRA-SET-7 | minor (needs triage: may be intentional) | **fixed (task 24) — GrokBot re-run EXTRA-SET-7 pending** | Language dropdown offers only 5 of the 10 shipped locales; some strings stay English | Settings › Appearance › Language: open the dropdown; switch to Japanese and look at Settings › Keyboard and Appearance. | cs, de, en, es, et, ja, nb_NO, pt, ru, zh_Hans (gui-test-plan EXTRA-SET-7; `locales/` has a yaml file for each). | Only English, Chinese (Simplified), German, Spanish, Japanese (`Language::all()`, src/config/settings.rs:1134). The 5 offered apply live, have no raw keys or tofu, and persist (Japanese survived a restart). In every non-English UI "Use system title bar" and "Restart Ferrite for this change to take effect." stay English; in Japanese several Keyboard labels fall back to English (Toggle Word Wrap, Lock editing, Toggle Terminal, Toggle Productivity Hub, Zoom In, Reset Zoom). | shots/EXTRA-SET-7-zh_Hans.png, shots/EXTRA-SET-7-ja.png; logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log, logs/ferrite-debug-20261009-052638-EXTRA-SET-7-ja-restart.log |
| ISS-57 | MMD-3 | minor | **fixed (`5eec844`) — GrokBot re-run MMD-3 pending** | Mermaid: indexed `linkStyle 0,1,2 interpolate basis` has no effect (`linkStyle default interpolate basis` works) | 1. `test_md/test_mermaid_v031.md` §3, Rendered. 2. Compare the same graph with no linkStyle, with `linkStyle 0,1,2,3,4 interpolate basis`, and with `linkStyle default interpolate basis`. | The indexed edges are drawn as curves. | The indexed form renders pixel-identical to no linkStyle (square elbows); the `default` form draws smooth curves. No error shown. | shots/MMD-3-fail.png, shots/MMD-3-control-AB-crop.png, shots/MMD-3-controlC.png; logs/ferrite-debug-20261009-035014-MMD-1.log |
| ISS-58 | MMD-6 | minor | **fixed (`5eec844`) — GrokBot re-run MMD-6 pending** | Mermaid: an unknown node id in a `%% @pos` hint makes the whole diagram fall back to raw source | Open `test_md/test_pos_hints.md`, Rendered, and look at the invalid-hint block (`flowchart TD`, `X[Only node]`, `%% @pos Ghost …`). | Warning shown; the diagram still renders. | Amber header "Line 3: Unknown node id 'Ghost' in @pos hint", then the source as monospace text and no node box. Without the bad line the same graph renders. The warning is only in the UI (no log line). Valid hints work. | shots/MMD-6-fail.png, shots/MMD-6-control.png; logs/ferrite-debug-20261009-035014-MMD-1.log |
| ISS-59 | MMD-13 | minor | **fixed (task 17) — GrokBot re-run MMD-13 pending** | Mermaid flowchart: CJK node boxes are 65–70 % wider than their text, and box sizing is inconsistent | Open `test_md/test_mermaid_issue_165.md`, FC-165a, popup at 100 %, and compare box widths with label widths. | Boxes fit their labels consistently. | A row: 堡垒机运维系统 196 vs 116 px (1.69×), SDP零信任访问 192 vs 112 (1.71×), AI Agent命令代理 212 vs 130 (1.63×); other boxes 1.04–1.36×. The layout itself is stable across first paint and scroll-away/back. | shots/MMD-13-firstpaint-fit.png, shots/MMD-13-scrolled-away.png, shots/MMD-13-after-scroll-back.png, shots/MMD-11-popup-native-100pct.png; logs/ferrite-debug-20261009-035014-MMD-1.log |
| ISS-60 | MMD-10 | minor (doc/checklist mismatch + cosmetic) | **fixed (task 16) — GrokBot re-run MMD-10 pending** | Sequence diagrams: `autonumber off` only works as a whole-diagram switch; number badges overlap message text | 1. FC-165b in `test_md/test_mermaid_issue_165.md`, Rendered. 2. A sequence diagram with some messages, then `autonumber off`, then more messages. | Row MMD-10: numbering stops from the `off` line on. | FC-165b numbers 1–10 correctly and `autonumber 10 10` gives 10, 20, 30. `autonumber off` as the last directive removes every badge, including earlier ones. The repo doc `docs/technical/mermaid/sequence-autonumber-rendering.md` says only the last top-level directive applies and mid-diagram off is out of scope, so checklist and doc disagree. Cosmetic: badges 5 and 6 sit on the message text; the first label runs into the participant header. | shots/MMD-10-partial-fc165b-inline.png, shots/MMD-10-partial-control-autonumber-10-10-and-off.png; logs/ferrite-debug-20261009-040706-MMD-10-control.log |
| ISS-61 | TBL-3 | minor | **fixed (task 23) — GrokBot re-run TBL-3 pending** | Raw-view table column guides follow the header row only; body pipes don't line up (proportional font); no guides while word wrap is ON | 1. `test_md/test_tables.md`, Raw. 2. Alt+Z (word wrap OFF). 3. Look at "Wide Table (5 columns)". | Guides line up with the columns of every row. | Wrap ON (default): no guides (by design, `table_guides.rs`: `if wrap_enabled { return; }`). Wrap OFF: guides at the header's pipe positions; body pipes are ~20 px to the left and the guides run through body text (also `alignment_test.md` and the 2×2/3×3 tables). Viewing never dirties the document. | shots/TBL-3-tables-nowrap.png, shots/TBL-3-guides-nowrap.png; logs/ferrite-debug-20261009-041820-WS-1.log |
| ISS-62 | TBL-3 (side effect), T2-39 | minor (synthetic-input caveat) | **fixed (`5eec844`) — GrokBot re-run T2-39/TBL-3 pending** | Alt+Z toggles word wrap and also types a literal `z` into the Raw editor | Raw editor focused; press Alt+Z. | Word wrap toggles; no text is inserted. | TBL-3: wrap toggled (`Keyboard shortcut: Alt+Z (Toggle Word Wrap)`, 02:30:53Z) **and** a `z` appeared at the start of line 1 (tab dirty; Ctrl+Z removed it). In T2-39 three synthetic Alt+Z presses typed `z` without toggling. xdotool may deliver Alt differently from a real keyboard; please re-check by hand. | shots/T2-39-altz-attempt4.png; logs/ferrite-debug-20261009-041820-WS-1.log |
| ISS-63 | SES-6 (also SES-1, SES-3) | polish | **fixed (`5eec844`) — GrokBot re-run SES-6 dialog pending** | Window-close and app-exit unsaved-changes dialogs do not name the affected file(s) | 1. Dirty tab in a second window → close that window. 2. Dirty tab → Alt+F4. | The dialog names the file(s) it is about (row SES-6). | "This window has unsaved changes. Close anyway?" and "You have unsaved changes. Exit anyway?" with no file name. The dialogs act on the right window and tabs. The single-tab close dialog does name the file ("'ses1-b.md*' has unsaved changes"). | shots/SES-6-window2-dialog.png, shots/SES-6-window1-dialog.png, shots/SES-1-fail-dialog.png, shots/SES-3-pass-dialog.png; logs/ferrite-debug-20261009-024942-SES-6.log |

Other observations, not filed as issues:
- After `kill -9`, the next launches show a Session Recovery dialog, as expected.
- WARNs on every launch: winit XSETTINGS, XRandR "0mm", epaint replacement chars. These are environmental.
- The ribbon gear toggles Settings closed.
- T1-18b's first Save As produced `fixtures/t1-18b.md.md` (log: `Saving file as: …/t1-18b.md.md`). The results don't say whether the tester typed the extension, so this is unverified.
- Batch 4 noted that a title-bar click once left the window in move mode, and a top-right title-bar button toggled Zen Mode. These are unconfirmed and may be misclicks. Batch 5b saw the move-mode effect again: the first click on a Ferrite title bar moved the window by about 350 px.
- **No Quit command (Batches 5–6):** Ctrl+Q has no binding (no match in `src/`), and the File menu has no Quit item (no quit label in `locales/en.yaml`). Only Alt+F4 or the window close button quits. With two windows open, Alt+F4 on the root window quits and saves the session for both (T2-37).
- **No unsaved-changes prompt when closing a modified Untitled doc (needs confirmation):** in T3-46, Alt+F4 on a modified Untitled doc quit with no Unsaved Changes prompt (log: CloseRequested → closing root viewport, exit 0). The text went only into the crash-recovery session file (`path=None`), and the next launch restored it as `Untitled*` (T3-47). This may be an intended hot-exit design. In the same batch, closing the window with an edited *saved* fixture did show a dialog (T3-41 and T3-45 were discarded with Don't Save), as in T1-9/T1-10. In T2-37 and T2-39, window 2 and a tab also closed without a prompt, but the batch notes don't say whether they were modified.
- **Session-restore pre-scan count (T2-37 relaunch):** the log says "Restoring 14 tab(s)" and lists 14 paths, including `/tmp/lf-test.md` (closed earlier in that run) and four video docs that are not restored. Then it says "Restored 11 of 11 tabs from session (2 windows)", and the UI matches the saved session. Probably related to ISS-21.
- **`--new-instance` shares state with the primary (SI-3, worth triage):** the independent instance found the running primary's `session.lock`, logged "Found stale lock file - previous session may have crashed", and offered Session Recovery. "Start Fresh" logged "User discarded crash recovery" / "Cleared all session recovery data", which clears the primary's crash-recovery data. On exit it logged "Removed session lock file", so the primary's `session.lock` was gone, and it overwrote the shared `session.json` and `config.json` in `~/.config/ferrite`. The same Session Recovery prompt appeared for the 2nd process in SI-4.
- **"Allow multiple Ferrite windows/instances" is saved late (SI-4):** after toggling it, `config.json` stayed stale for 2+ minutes. The value was written only at the next `save_settings_if_dirty` event (a file open or close from the tree), and no periodic eframe save ran. If the app is killed in that window, the change is lost.
- **Ctrl+, again did not open Settings (SI-4)**; the gear button did. See ISS-12.
- **Silent crash-recovery restore once (T2-35):** after the first `kill -9`, the relaunch loaded the crash-recovery state with **no** Session Recovery dialog (log: "Found stale lock file", `is_crash_recovery=true`). The other kill -9 relaunches (e.g. SI-5) showed the dialog.
- T3-41: right after typing in a Rendered table cell, the first switch to Raw showed the old text and no `*` until a Rendered round trip; the edit had reached the buffer.
- T3-47: on this box `xdg-open` sends `.zip` and `.exe` to Chrome (xdg-mime `com.google.Chrome.desktop`), which downloaded `sample.zip`, `sample (1).zip` and `FerriteMDPortable.exe` into `~/Downloads` (outside the repo; nothing was run). Once, the Ferrite window was found minimised after a re-click and was restored with `xdotool windowactivate`; no data was lost.
- **Batches 7–12 (Oct 9) additions:**
  - **IND-7 anomaly:** a multi-cursor attempt removed the text of lines 7–9 (1467 → 1424 chars); Ctrl+Z restored the buffer exactly. Not reproduced; may be a stray synthetic keystroke.
  - **IND-8:** the first Ctrl+S did not write the file; a second save did. Not investigated.
  - **GIT-3 reload after own save:** each save of the new untracked file logged `Reloaded externally modified file` right after `Saved file` (2×, 02:54:28Z and 02:54:54Z); the tracked-file save did not. One save ran while typing was still going, so the disk got a partial text and the buffer was reloaded before the rest of the typed text arrived; the second save wrote the full text. Probably a timing effect of the scripted typing (harness), but a watcher reacting to Ferrite's own save is worth a look.
  - **EXTRA-SET-6 capture:** while capturing, `Ctrl+Shift+K (Image)` still fired, and a plain letter (`K`) could be bound without modifiers. Both came from the Alt-loss attempts and may be harness-related.
  - **CSV-4 wording:** the large-CSV banner says "Cell editing is disabled — switch to Raw view (Ctrl+E) to edit", and a second red banner says "Showing raw content" while the grid is still visible. Cosmetic mismatch.
  - **TBL-2:** a cell in edit mode is drawn left-aligned even when its column is centred or right-aligned; it re-aligns after commit.
  - **HTML-4 side note:** the heading `## Phase 1 — <br> line breaks` shows a box glyph (□), and the heading `## Phase 2 — <kbd>` drops the `<kbd>` text.
  - **SES-3:** the native Open dialog fails on this box (rfd xdg-desktop-portal `pick_files error No such file or directory`), so files were opened by single-instance forwarding instead.
  - **SPL-3:** the spellcheck hint tooltip covers the right-click context menu, so suggestion clicks fall through (row recorded as BLOCKED).
  - **Ctrl+Shift+N:** the first press(es) were often lost (SES-3: 1 press, EXTRA-SET-13: 4 presses) until the editor was clicked.
  - **Click-through (SIF-4):** clicks on the Find panel's own toggles and field also reached the tab strip underneath and switched tabs; keyboard-only input worked.
  - **FNT-4:** the Ferrite terminal runs `/bin/sh` (dash on Debian), so `printf '\u…'` prints literal text; octal `printf` and `python3` were used instead.
  - **Silent session restores:** many launches restored about 30–40 tabs from the shared session with no Session Recovery dialog. This slowed tests but caused no failure.

### Log excerpts per issue

#### ISS-1: deadlock (hang backtraces + debug-log tails)

Backtrace excerpt, `logs/T1-2-hang-bt.txt`, main thread (commit-on-click-away path):

```text
Thread 1 (Thread 0x7fb0a563c300 (LWP 130676) "ferrite"):
#0  0x00007fb0b64024f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x0000560373e72249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x00005603738e6b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x00005603738f3434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x00005603738f8552 in ferrite::markdown::editor::render_session_plain_text_block ()
#5  0x00005603734e1527 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#6  0x0000560373642c94 in egui::ui::Ui::horizontal_with_main_wrap_dyn ()
#7  0x00005603738e61d9 in ferrite::markdown::editor::render_paragraph ()
#8  0x00005603738d6254 in ferrite::markdown::editor::render_node ()
#9  0x00005603735180a8 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#10 0x0000560373642525 in egui::ui::Ui::allocate_ui_with_layout_dyn ()
#11 0x0000560373547ac7 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#12 0x0000560373642e5c in egui::ui::Ui::scope_dyn ()
#13 0x0000560373513228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#14 0x0000560373642e5c in egui::ui::Ui::scope_dyn ()
#15 0x00005603734d3278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#16 0x00005603738e2242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#17 0x00005603738e4faa in ferrite::markdown::editor::MarkdownEditor::show ()
#18 0x00005603737c3a90 in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
```

Backtrace excerpt, `logs/T1-17-retry-hang-bt.txt` (single ArrowDown path; T1-17 and T1-18 are identical):

```text
Thread 1 (Thread 0x7f42f590c300 (LWP 183748) "ferrite"):
#0  0x00007f43066d24f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000056088b191249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000056088ac05b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000056088ac12434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000056088a8680ae in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#5  0x000056088a961e5c in egui::ui::Ui::scope_dyn ()
#6  0x000056088a832228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#7  0x000056088a961e5c in egui::ui::Ui::scope_dyn ()
#8  0x000056088a7f2278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x000056088ac01242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#10 0x000056088ac03faa in ferrite::markdown::editor::MarkdownEditor::show ()
#11 0x000056088aae2a90 in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
#12 0x000056088a9310ff in egui::containers::panel::CentralPanel::show_inside_dyn ()
#13 0x000056088aabbe1d in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel ()
#14 0x000056088aa9b3f2 in ferrite::app::FerriteApp::render_ui ()
#15 0x000056088a98fab2 in <ferrite::app::FerriteApp as eframe::epi::App>::ui ()
```

Backtrace excerpt, `logs/T1-22-hang-bt.txt` (formatted-paragraph edit path):

```text
Thread 1 (Thread 0x7f132bd1c300 (LWP 250155) "ferrite"):
#0  0x00007f133cae24f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000055f46ddfe249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000055f46d872b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000055f46d87f434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000055f46d887509 in ferrite::markdown::editor::render_session_formatted_edit_text ()
#5  0x000055f46d52c2a4 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#6  0x000055f46d5cec94 in egui::ui::Ui::horizontal_with_main_wrap_dyn ()
#7  0x000055f46d8723f4 in ferrite::markdown::editor::render_paragraph ()
#8  0x000055f46d862254 in ferrite::markdown::editor::render_node ()
#9  0x000055f46d4a40a8 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#10 0x000055f46d5ce525 in egui::ui::Ui::allocate_ui_with_layout_dyn ()
#11 0x000055f46d4d3ac7 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#12 0x000055f46d5cee5c in egui::ui::Ui::scope_dyn ()
#13 0x000055f46d49f228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#14 0x000055f46d5cee5c in egui::ui::Ui::scope_dyn ()
#15 0x000055f46d45f278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#16 0x000055f46d86e242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#17 0x000055f46d870faa in ferrite::markdown::editor::MarkdownEditor::show ()
```

The hung logs end with periodic crash-recovery snapshots or IME events, then silence until the kill line. Each was killed with kill -9, so the logs end with exit 137.

<details><summary>T1-2 (final attempt, HANG-A): logs/ferrite-debug-20261008-203933-HANG-A.log</summary>

```text
[2026-10-08T18:39:54Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:39:54Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:39:54Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:39:54Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:39:54Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1020.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T18:39:55Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T18:39:55Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:40:03Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:40:03Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T18:40:03Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 105, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 109, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 109, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 117, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 117, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 126, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 126, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 129, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 129, height: 17 })
[2026-10-08T18:40:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:40:09Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:40:09Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 142, height: 17 })
[2026-10-08T18:40:09Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 142, height: 17 })
[2026-10-08T18:40:09Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 150, height: 17 })
[2026-10-08T18:40:09Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 146 }) size=Physical(PhysicalSize { width: 150, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 20:44:06 CEST ===
```
</details>

<details><summary>T1-2 (attempt 2): logs/ferrite-debug-20261008-201751-T1-2-retry.log</summary>

```text
[2026-10-08T18:17:59Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:17:59Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:17:59Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:17:59Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:17:59Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T18:18:00Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T18:18:00Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:18:06Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:18:06Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T18:18:06Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 105, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 109, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 109, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 117, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 117, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 126, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 126, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 129, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 129, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 142, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 142, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 150, height: 17 })
[2026-10-08T18:18:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 150, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 20:19:23 CEST ===
```
</details>

<details><summary>T1-1/T1-2 (attempt 1, session1): logs/ferrite-debug-20261008-201237-session1.log</summary>

```text
[2026-10-08T18:13:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:34Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:34Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:35Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:35Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:36Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:36Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:37Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:37Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:13:37Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T18:13:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T18:13:38Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T18:13:38Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:13:45Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:13:45Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T18:13:45Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 146 }) size=Physical(PhysicalSize { width: 105, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 20:17:35 CEST ===
```
</details>

<details><summary>T1-7 (HANG-B): logs/ferrite-debug-20261008-204439-HANG-B.log</summary>

```text
[2026-10-08T18:44:50Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:44:50Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:44:56Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:44:56Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T18:44:56Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 127, height: 17 })
[2026-10-08T18:45:01Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:45:01Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 134, height: 17 })
[2026-10-08T18:45:01Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 143, height: 17 })
[2026-10-08T18:45:01Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 143, height: 17 })
[2026-10-08T18:45:08Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+R (Reload from Disk)
[2026-10-08T18:45:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 143, height: 17 })
[2026-10-08T18:45:08Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T18:45:08Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=SingleFile, windows=1, tabs=6, clean_shutdown=false
[2026-10-08T18:45:08Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:45:08Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:45:08Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 6 tabs, SingleFile)
[2026-10-08T18:45:08Z DEBUG ferrite::config::session] Saved recovery content for tab 8 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(7344225401572943942))
[2026-10-08T18:45:08Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:45:08Z DEBUG winit::window] winit::Window::set_title; title="reload-test.md* - Ferrite"
[2026-10-08T18:45:08Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 143, height: 17 })
[2026-10-08T18:45:13Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T18:45:13Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=SingleFile, windows=1, tabs=6, clean_shutdown=false
[2026-10-08T18:45:13Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:45:13Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:45:13Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 6 tabs, SingleFile)
[2026-10-08T18:45:13Z DEBUG ferrite::config::session] Saved recovery content for tab 8 (path=Some("/workspace/ferrite-qa/fixtures/reload-test.md"), hash=Some(7344225401572943942))
[2026-10-08T18:45:13Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:45:14Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:45:14Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 174, y: 181 }) size=Physical(PhysicalSize { width: 143, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 20:47:15 CEST ===
```
</details>

<details><summary>T1-7 (first occurrence; same process as T1-4b/5/6): logs/ferrite-debug-20261008-202232-T1-4b.log</summary>

```text
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
```
</details>

<details><summary>T1-17 (first attempt; same process as T1-12…T1-16b): logs/ferrite-debug-20261008-204753-batch2.log</summary>

```text
[2026-10-08T18:59:23Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T18:59:23Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:59:26Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=11, clean_shutdown=false
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 11 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Saved recovery content for tab 7 (path=None, hash=None)
[2026-10-08T18:59:26Z DEBUG ferrite::config::session] Saved recovery content for tab 8 (path=Some("/workspace/ferrite-qa/fixtures/t1-multibyte.md"), hash=Some(9378801994929134664))
[2026-10-08T18:59:26Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:59:30Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:59:30Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T18:59:30Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 139 }) size=Physical(PhysicalSize { width: 765, height: 31 })
[2026-10-08T18:59:31Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=11, clean_shutdown=false
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 11 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Saved recovery content for tab 7 (path=None, hash=None)
[2026-10-08T18:59:31Z DEBUG ferrite::config::session] Saved recovery content for tab 8 (path=Some("/workspace/ferrite-qa/fixtures/t1-multibyte.md"), hash=Some(9378801994929134664))
[2026-10-08T18:59:31Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T18:59:36Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=11, clean_shutdown=false
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 11 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Saved recovery content for tab 7 (path=None, hash=None)
[2026-10-08T18:59:36Z DEBUG ferrite::config::session] Saved recovery content for tab 8 (path=Some("/workspace/ferrite-qa/fixtures/t1-multibyte.md"), hash=Some(9378801994929134664))
[2026-10-08T18:59:36Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 21:02:25 CEST ===
```
</details>

<details><summary>T1-17 retry (single ArrowDown): logs/ferrite-debug-20261008-210253-batch2.log</summary>

```text
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:27Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:27Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:05:28Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:05:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:05:28Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:05:28Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 131 }) size=Physical(PhysicalSize { width: 810, height: 615 })
[2026-10-08T19:05:28Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T19:05:28Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T19:05:37Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:05:37Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T19:05:37Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 139 }) size=Physical(PhysicalSize { width: 765, height: 31 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 21:06:30 CEST ===
```
</details>

<details><summary>T1-18: logs/ferrite-debug-20261008-210656-batch2c.log</summary>

```text
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:34Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:34Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:35Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:35Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:35Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:35Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:36Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:36Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:07:36Z DEBUG ferrite::editor::widget] Syncing 9 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:07:36Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:07:37Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T19:07:37Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:07:37Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T19:07:44Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:07:44Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T19:07:44Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 607 }) size=Physical(PhysicalSize { width: 480, height: 34 })
[2026-10-08T19:07:50Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 607 }) size=Physical(PhysicalSize { width: 480, height: 34 })
[2026-10-08T19:07:50Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 607 }) size=Physical(PhysicalSize { width: 480, height: 34 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 21:08:54 CEST ===
```
</details>

<details><summary>T1-18b: logs/ferrite-debug-20261008-210920-batch2d.log</summary>

```text
[2026-10-08T19:11:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:05Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:05Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:11:06Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T19:11:06Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-08T19:11:06Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:11:06Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T19:11:21Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:11:21Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-08T19:11:21Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 335, y: 220 }) size=Physical(PhysicalSize { width: 81, height: 34 })
[2026-10-08T19:11:32Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 335, y: 220 }) size=Physical(PhysicalSize { width: 81, height: 34 })
[2026-10-08T19:11:40Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 335, y: 220 }) size=Physical(PhysicalSize { width: 89, height: 34 })
[2026-10-08T19:11:40Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 335, y: 220 }) size=Physical(PhysicalSize { width: 89, height: 34 })
[2026-10-08T19:11:47Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:11:47Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 335, y: 220 }) size=Physical(PhysicalSize { width: 89, height: 34 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 21:12:36 CEST ===
```
</details>

<details><summary>T1-22 session 1 (hang): logs/ferrite-debug-20261008-213134-T1-22.log</summary>

```text
[2026-10-08T19:33:25Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T19:33:25Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T19:33:25Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 14 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:33:25Z DEBUG ferrite::config::session] Saved recovery content for tab 25 (path=None, hash=None)
[2026-10-08T19:33:25Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T19:33:30Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T19:33:30Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=14, clean_shutdown=false
[2026-10-08T19:33:30Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T19:33:30Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T19:33:30Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 14 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:33:30Z DEBUG ferrite::config::session] Saved recovery content for tab 25 (path=None, hash=None)
[2026-10-08T19:33:30Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T19:33:35Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T19:33:35Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=14, clean_shutdown=false
[2026-10-08T19:33:35Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T19:33:35Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T19:33:35Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 14 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:33:35Z DEBUG ferrite::config::session] Saved recovery content for tab 25 (path=None, hash=None)
[2026-10-08T19:33:35Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T19:33:40Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T19:33:40Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=14, clean_shutdown=false
[2026-10-08T19:33:40Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-08T19:33:40Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-08T19:33:40Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 14 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:33:40Z DEBUG ferrite::config::session] Saved recovery content for tab 25 (path=None, hash=None)
[2026-10-08T19:33:40Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-08T19:33:41Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:33:41Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 283 }) size=Physical(PhysicalSize { width: 554, height: 34 })
[2026-10-08T19:33:41Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 299, y: 283 }) size=Physical(PhysicalSize { width: 554, height: 34 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 21:36:19 CEST ===
```
</details>


#### ISS-2: GDK panic (last ~40 non-span lines: trigger + panic + backtrace + exit)

<details><summary>T2-36 (Rendered): logs/ferrite-debug-20261008-223058-T2-36.log</summary>

```text
[2026-10-08T20:31:23Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:31:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:31:23Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (407560) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:31:23Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:31:23Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:31:23Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:31:23 CEST ===
```
</details>

<details><summary>T2-36 retry (Rendered): logs/ferrite-debug-20261008-223148-T2-36-retry.log</summary>

```text
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:32:05Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1780, height: 1041 })
[2026-10-08T20:32:05Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (408904) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:32:05Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:32:05Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:32:05Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:32:05 CEST ===
```
</details>

<details><summary>T2-36 (Split): logs/ferrite-debug-20261008-223237-T2-36-split.log</summary>

```text
[2026-10-08T20:32:51Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:32:51Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:32:51Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3

thread 'main' (410247) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:32:51Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:32:51Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:32:51Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:32:51 CEST ===
```
</details>

<details><summary>VID-1: logs/ferrite-debug-20261008-223520-VID-1.log</summary>

```text
[2026-10-08T20:35:37Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:35:37Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:35:37Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (414195) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:35:37Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:35:37Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:35:37Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:35:37 CEST ===
```
</details>

<details><summary>EXTRA-YT-4: logs/ferrite-debug-20261008-223923-EXTRA-YT-4.log</summary>

```text
[2026-10-08T20:39:37Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:39:37Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 105 }) size=Physical(PhysicalSize { width: 1140, height: 641 })
[2026-10-08T20:39:37Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (422615) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:39:37Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:39:37Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:39:37Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:39:37 CEST ===
```
</details>

<details><summary>EXTRA-YT-6 (typed embed in Split): logs/ferrite-debug-20261008-223957-EXTRA-YT-6.log</summary>

```text
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:42:22Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=526.3, max_wrap_width=840.0, effective=526.3
[2026-10-08T20:42:22Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 25) -> (0, 26)

thread 'main' (423503) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:42:22Z DEBUG ureq::stream] connecting to img.youtube.com:443 at 198.18.0.1:443
[2026-10-08T20:42:22Z DEBUG rustls::client::hs] No cached session for DnsName("img.youtube.com")
[2026-10-08T20:42:22Z DEBUG rustls::client::hs] Not resuming any session
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:42:22 CEST ===
```
</details>

<details><summary>EXTRA-YT-2 (offline): logs/ferrite-debug-20261008-224251-EXTRA-YT-2.log</summary>

```text
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:43:28Z DEBUG ferrite::editor::widget] Syncing 12 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:43:28Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1100.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:43:28Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered

thread 'main' (428360) panicked at /home/box/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gdk-0.18.2/src/auto/display.rs:301:9:
GDK has not been initialized. Call `gdk::init` or `gtk::init` first.
stack backtrace:
[2026-10-08T20:43:28Z WARN  ferrite::markdown::video_render] Failed to fetch video thumbnail 'https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg': https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg: Dns Failed: resolve dns name 'img.youtube.com:443': failed to lookup address information: Tempora …[truncated]
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 22:43:28 CEST ===
```
</details>


#### ISS-3: UTF-16 LE delegated as binary (T1-4b)

Lines around the failure (non-span lines 101–104 of `logs/ferrite-debug-20261008-202232-T1-4b.log`):

```text
[2026-10-08T18:22:33Z INFO  ferrite::app] Opening file from CLI: /tmp/crlf-utf16.md
[2026-10-08T18:22:33Z INFO  ferrite::state] Delegating to external app (binary): /tmp/crlf-utf16.md
[2026-10-08T18:23:14Z INFO  ferrite::state] Opened in default application: /tmp/crlf-utf16.md
[2026-10-08T18:23:14Z INFO  ferrite::app] CLI initialization complete: 1 files opened, workspace mode active
```

The last 30 lines of the same process are under ISS-1, "T1-7 (first occurrence)". That instance went on to run T1-5…T1-7 and was killed in the T1-7 hang.


#### ISS-4: Search in Files caret (T1-15)

Lines around the click on the result (`logs/ferrite-debug-20261008-204753-batch2.log`, non-span lines 9893–9908; the same sequence repeats at 18:55:47Z):

```text
[2026-10-08T18:54:49Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T18:54:49Z INFO  ferrite::state] Settings saved
[2026-10-08T18:54:49Z DEBUG ferrite::app::file_ops] Opened file from search: /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md at line 1, char offset 5
[2026-10-08T18:54:49Z DEBUG ferrite::app::file_ops] Set transient highlight at 5..7 and scroll to line 1
[2026-10-08T18:54:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:54:49Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-08T18:54:49Z DEBUG ferrite::app] Deferred CJK check: loading fonts for newly opened file
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] Creating new FerriteEditor for tab 9
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] EditorWidget: scroll_to_line 1 (0-indexed: 0), total_lines=2, cursor_before=Cursor { line: 0, column: 0 }
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] EditorWidget: after set_cursor, cursor_after=Cursor { line: 0, column: 0 }
[2026-10-08T18:54:49Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=770.3, max_wrap_width=840.0, effective=770.3
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] EditorWidget: auto-focused tab 9
[2026-10-08T18:54:49Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 5) -> (0, 0)
[2026-10-08T18:54:49Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T18:54:49Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
```

The last 30 lines of this process are under ISS-1, "T1-17 (first attempt)". The session continued and ended in the T1-17 hang.


#### ISS-5: Ctrl+V image paste (T1-14)

All clipboard-related lines in `logs/ferrite-debug-20261008-204753-batch2.log` (non-span):

```text
[2026-10-08T18:51:44Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-08T18:52:47Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-08T18:59:01Z DEBUG ferrite::app::input_handling] Smart paste: Created link [hello](https://example.com) at char 69..74
```

The two `arboard paste error` lines are the two Ctrl+V presses. The last line is T1-16b's text smart paste, which worked. The last 30 lines are under ISS-1, "T1-17 (first attempt)".


#### ISS-6: split-view undo (T1-19)

Undo-related lines in `logs/ferrite-debug-20261008-211300-batch2e.log` (non-span; the 2nd edit produced 15 `recorded for undo` lines 19:16:08–09Z, abbreviated to 3 here):

```text
[2026-10-08T19:14:13Z DEBUG ferrite::app::central_panel] Content modified in split rendered pane, recorded for undo
[2026-10-08T19:14:13Z DEBUG ferrite::app::central_panel] Content modified in split rendered pane, recorded for undo
[2026-10-08T19:14:56Z DEBUG ferrite::app::input_handling] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-10-08T19:14:56Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
[2026-10-08T19:15:16Z DEBUG ferrite::app::navigation] Undo performed, 0 entries remaining
[2026-10-08T19:16:08Z DEBUG ferrite::app::central_panel] Content modified in split rendered pane, recorded for undo
[2026-10-08T19:16:08Z DEBUG ferrite::app::central_panel] Content modified in split rendered pane, recorded for undo
[2026-10-08T19:16:08Z DEBUG ferrite::app::central_panel] Content modified in split rendered pane, recorded for undo
[2026-10-08T19:16:56Z DEBUG ferrite::app::navigation] Undo performed, 14 entries remaining
[2026-10-08T19:17:13Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-10-08T19:17:14Z DEBUG ferrite::app::navigation] Undo performed, 14 entries remaining
[2026-10-08T19:17:30Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-10-08T19:17:47Z DEBUG ferrite::app::navigation] Undo performed, 14 entries remaining
[2026-10-08T19:17:47Z DEBUG ferrite::app::navigation] Redo performed, 0 entries remaining
[2026-10-08T19:17:47Z DEBUG ferrite::app::navigation] Undo performed, 14 entries remaining
[2026-10-08T19:17:47Z DEBUG ferrite::app::navigation] Undo performed, 13 entries remaining
[2026-10-08T19:17:47Z DEBUG ferrite::app::navigation] Redo performed, 1 entries remaining
```

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-211300-batch2e.log (session ended with a clean exit 0)</summary>

```text
[2026-10-08T19:20:41Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 131 }) size=Physical(PhysicalSize { width: 401, height: 615 })
[2026-10-08T19:20:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:20:41Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:20:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T19:20:41Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:20:41Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:20:41Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:20:41Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T19:20:41Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T19:20:41Z INFO  ferrite::app] Application exiting
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Deleted recovery content for tab 21
[2026-10-08T19:20:41Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=11, clean_shutdown=true
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 11 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:20:41Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T19:20:41Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T19:20:41Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (0 recent files, 1 expanded paths)
[2026-10-08T19:20:41Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T19:20:41Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T19:20:41Z INFO  ferrite::state] Settings saved
[2026-10-08T19:20:41Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T19:20:41Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T19:20:41Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T19:20:41Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T19:20:41Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:20:41Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T19:20:41Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 21:20:41 CEST ===
```
</details>


#### ISS-7: backtick auto-close while typing a fence (T1-21)

Excerpt from the first typed run (`logs/ferrite-debug-20261008-212111-T1-20.log`, non-span lines 11715–11790, wrap/fold noise removed). Every keystroke is followed by an auto-close insert, so the content grows by 2 while the caret advances by 1:

```text
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 3) -> (0, 4)
[2026-10-08T19:26:38Z DEBUG ferrite::app::input_handling] Auto-close: Inserted '`' after '`'
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget sync: length changed (Some(5) -> 6)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: needs_content_sync=true for tab 24 (reason: length_changed)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] Syncing Tab content to FerriteEditor for tab 24 (content_len=6, scroll_offset=0.0, cursor=(0,4))
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 4) -> (0, 5)
[2026-10-08T19:26:38Z DEBUG ferrite::app::input_handling] Auto-close: Inserted '`' after '`'
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget sync: length changed (Some(7) -> 8)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: needs_content_sync=true for tab 24 (reason: length_changed)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] Syncing Tab content to FerriteEditor for tab 24 (content_len=8, scroll_offset=0.0, cursor=(0,5))
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 5) -> (0, 6)
[2026-10-08T19:26:38Z DEBUG ferrite::app::input_handling] Auto-close: Inserted '`' after '`'
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget sync: length changed (Some(9) -> 10)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: needs_content_sync=true for tab 24 (reason: length_changed)
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] Syncing Tab content to FerriteEditor for tab 24 (content_len=10, scroll_offset=0.0, cursor=(0,6))
[2026-10-08T19:26:38Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 6) -> (0, 7)
```

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-212111-T1-20.log (T1-20 + T1-21 session, clean exit 0, no panic)</summary>

```text
[2026-10-08T19:31:00Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T19:31:00Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 250, y: 157 }) size=Physical(PhysicalSize { width: 401, height: 589 })
[2026-10-08T19:31:00Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:31:00Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:31:00Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T19:31:00Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T19:31:00Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T19:31:00Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=361.3, max_wrap_width=840.0, effective=361.3
[2026-10-08T19:31:00Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T19:31:00Z INFO  ferrite::app] Application exiting
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Saved recovery content for tab 24 (path=None, hash=None)
[2026-10-08T19:31:00Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=13, clean_shutdown=true
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 13 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-08T19:31:00Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T19:31:00Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T19:31:00Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (0 recent files, 1 expanded paths)
[2026-10-08T19:31:00Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T19:31:00Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T19:31:00Z INFO  ferrite::state] Settings saved
[2026-10-08T19:31:00Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T19:31:00Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T19:31:00Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T19:31:00Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T19:31:00Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:31:00Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T19:31:00Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 21:31:00 CEST ===
```
</details>


#### ISS-8: Mermaid popup / Fit-Native (T1-22)

<details><summary>Session 2 (no freeze, popup never opened): logs/ferrite-debug-20261008-213643-T1-22b.log, last 30 non-span lines (this process went on to run T2-23…T2-25 and was quit cleanly)</summary>

```text
[2026-10-08T19:44:44Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:44:57Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T19:44:58Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T19:44:58Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T19:44:58Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T19:44:58Z INFO  ferrite::state] Settings saved
[2026-10-08T19:44:58Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T19:44:58Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T19:44:58Z INFO  ferrite::app] Application exiting
[2026-10-08T19:44:58Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T19:44:58Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=14, clean_shutdown=true
[2026-10-08T19:44:58Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T19:44:58Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T19:44:58Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 14 tabs, SingleFile)
[2026-10-08T19:44:58Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T19:44:58Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T19:44:58Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T19:44:58Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T19:44:58Z INFO  ferrite::state] Settings saved
[2026-10-08T19:44:58Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T19:44:58Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T19:44:58Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T19:44:58Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T19:44:58Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:44:58Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:44:58Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:44:58Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T19:44:58Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T19:44:58Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 21:44:58 CEST ===
```
</details>

Session 1 (hang): see ISS-1, "T1-22 session 1".


#### ISS-9: corrupt config silently reset (EXTRA-SET-9)

Relevant lines:

```text
# ferrite-debug-20261008-221551-EXTRA-SET-9a.log
[2026-10-08T20:15:51Z WARN  ferrite::config::persistence] Config file at /home/box/.config/ferrite/config.json contains invalid JSON: key must be a string at line 1 column 3
[2026-10-08T20:15:51Z WARN  ferrite::error] Failed to load configuration: Invalid configuration format: Failed to parse config file: key must be a string at line 1 column 3. Using default.
[2026-10-08T20:16:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:16:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
=== Ferrite exited with code 0 clean exit at 22:16:31 CEST ===
# ferrite-debug-20261008-221645-EXTRA-SET-9b.log
[2026-10-08T20:16:46Z WARN  ferrite::config::persistence] Config file at /home/box/.config/ferrite/config.json contains invalid JSON: invalid type: string "abc", expected f32 at line 1 column 117
[2026-10-08T20:16:46Z WARN  ferrite::error] Failed to load configuration: Invalid configuration format: Failed to parse config file: invalid type: string "abc", expected f32 at line 1 column 117. Using default.
[2026-10-08T20:17:14Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:17:14Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
=== Ferrite exited with code 0 clean exit at 22:17:14 CEST ===
```

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-221551-EXTRA-SET-9a.log</summary>

```text
[2026-10-08T20:16:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:16:31Z INFO  ferrite::state] Settings saved
[2026-10-08T20:16:31Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:16:31Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:16:31Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:16:31Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:16:31Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T20:16:31Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 131 }) size=Physical(PhysicalSize { width: 1780, height: 1015 })
[2026-10-08T20:16:31Z INFO  ferrite::app] Application exiting
[2026-10-08T20:16:31Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T20:16:31Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=14, clean_shutdown=true
[2026-10-08T20:16:31Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T20:16:31Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T20:16:31Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 14 tabs, SingleFile)
[2026-10-08T20:16:31Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T20:16:31Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T20:16:31Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:16:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:16:31Z INFO  ferrite::state] Settings saved
[2026-10-08T20:16:31Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:16:31Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T20:16:31Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T20:16:31Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T20:16:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:16:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:16:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:16:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:16:31Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T20:16:31Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 22:16:31 CEST ===
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-221645-EXTRA-SET-9b.log</summary>

```text
[2026-10-08T20:17:14Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:17:14Z INFO  ferrite::state] Settings saved
[2026-10-08T20:17:14Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:17:14Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-08T20:17:14Z DEBUG ferrite::editor::widget] Syncing 2 fold regions to FerriteEditor, show_indicators=false
[2026-10-08T20:17:14Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1740.3, max_wrap_width=840.0, effective=840.0
[2026-10-08T20:17:14Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T20:17:14Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 131 }) size=Physical(PhysicalSize { width: 1780, height: 1015 })
[2026-10-08T20:17:14Z INFO  ferrite::app] Application exiting
[2026-10-08T20:17:14Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T20:17:14Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=14, clean_shutdown=true
[2026-10-08T20:17:14Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T20:17:14Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T20:17:14Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 14 tabs, SingleFile)
[2026-10-08T20:17:14Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T20:17:14Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T20:17:14Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:17:14Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:17:14Z INFO  ferrite::state] Settings saved
[2026-10-08T20:17:14Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:17:14Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T20:17:14Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T20:17:14Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T20:17:14Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:17:14Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:17:14Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:17:14Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:17:14Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T20:17:14Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 22:17:14 CEST ===
```
</details>


#### ISS-10: invisible text with ANSI syntax theme (EXTRA-SET-2-defect)

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-220315-EXTRA-SET-2-relaunch.log (no WARN/ERROR tied to the defect; it is purely visual)</summary>

```text
[2026-10-08T20:07:46Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:07:46Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:08:06Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T20:08:06Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:08:06Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:08:06Z INFO  ferrite::state] Settings saved
[2026-10-08T20:08:06Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:08:06Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T20:08:06Z INFO  ferrite::app] Application exiting
[2026-10-08T20:08:06Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T20:08:06Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=14, clean_shutdown=true
[2026-10-08T20:08:06Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T20:08:06Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T20:08:06Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 14 tabs, SingleFile)
[2026-10-08T20:08:06Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T20:08:06Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T20:08:06Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:08:06Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:08:06Z INFO  ferrite::state] Settings saved
[2026-10-08T20:08:06Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:08:06Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T20:08:06Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T20:08:06Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T20:08:06Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:08:06Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:08:06Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:08:06Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:08:06Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T20:08:06Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 22:08:06 CEST ===
```
</details>


#### ISS-11: Recent Files 0 not persisted (EXTRA-SET-4)

```text
[2026-10-08T20:28:54Z DEBUG ferrite::state] Session restore disabled in settings, skipping tab restoration
```

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-222853-set-persist.log</summary>

```text
[2026-10-08T20:29:50Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-08T20:30:30Z DEBUG ferrite::vcs::git] Window focus gained, requesting git refresh
[2026-10-08T20:30:31Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T20:30:31Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:30:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:30:31Z INFO  ferrite::state] Settings saved
[2026-10-08T20:30:31Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:30:31Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T20:30:31Z INFO  ferrite::app] Application exiting
[2026-10-08T20:30:31Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T20:30:31Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=1, clean_shutdown=true
[2026-10-08T20:30:31Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T20:30:31Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T20:30:31Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 1 tabs, SingleFile)
[2026-10-08T20:30:31Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T20:30:31Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T20:30:31Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:30:31Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:30:31Z INFO  ferrite::state] Settings saved
[2026-10-08T20:30:31Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:30:31Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T20:30:31Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T20:30:31Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T20:30:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:30:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:30:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:30:31Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:30:31Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T20:30:31Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 22:30:31 CEST ===
```
</details>


#### ISS-12: Esc / Ctrl+, (EXTRA-SET-1)

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-221729-EXTRA-SET-10.log (same process as EXTRA-SET-1/10/11/12; no WARN/ERROR for these actions)</summary>

```text
[2026-10-08T20:27:28Z DEBUG ferrite::theme::manager] Applied theme: Light
[2026-10-08T20:27:28Z DEBUG ferrite::theme::manager] Applied theme: Light
[2026-10-08T20:28:35Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-08T20:28:35Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:28:35Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:28:35Z INFO  ferrite::state] Settings saved
[2026-10-08T20:28:35Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:28:35Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-08T20:28:35Z INFO  ferrite::app] Application exiting
[2026-10-08T20:28:35Z DEBUG ferrite::state] Capturing session state in single-file mode
[2026-10-08T20:28:35Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=SingleFile, windows=1, tabs=14, clean_shutdown=true
[2026-10-08T20:28:35Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-08T20:28:35Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-08T20:28:35Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 14 tabs, SingleFile)
[2026-10-08T20:28:35Z INFO  ferrite::app] Session state saved for next startup
[2026-10-08T20:28:35Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-08T20:28:35Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-08T20:28:35Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-08T20:28:35Z INFO  ferrite::state] Settings saved
[2026-10-08T20:28:35Z INFO  ferrite::state] AppState shutdown complete
[2026-10-08T20:28:35Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-08T20:28:35Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-08T20:28:35Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-08T20:28:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:28:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:28:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:28:35Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-08T20:28:35Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-08T20:28:35Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 22:28:35 CEST ===
```
</details>


#### ISS-30: emoji (T2-32)

Font set-up lines in `logs/ferrite-debug-20261008-230020-T2-32.log` (non-span, `ferrite::fonts`): the colour emoji font is found and registered, yet glyphs are drawn monochrome and two are blank.

```text
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system emoji font: Noto Color Emoji
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Found system emoji font at: "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf"
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Registered system emoji font fallback
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Loaded fonts: CJK(KR=false, JP=false, SC=false, TC=false), ComplexScript=false, customs=[]
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Font generation bumped to 1
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Configured fonts in lazy mode (CJK deferred)
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Font generation bumped to 2
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Pre-warmed font atlas with 41 box-drawing, 43 symbol, and 9 emoji characters
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Lazily loading CJK fonts for detected scripts: Korean=true, Japanese=true, Han=true
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Apple SD Gothic Neo
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Malgun Gothic
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Noto Sans CJK KR
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Found system font at: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Loaded Korean font (first time)
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Hiragino Sans
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Hiragino Kaku Gothic ProN
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Yu Gothic
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Meiryo
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Noto Sans CJK JP
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Found system font at: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Loaded Japanese font (first time)
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: PingFang SC
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Microsoft YaHei
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Attempting to load system font: Noto Sans CJK SC
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Found system font at: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Loaded Simplified Chinese font (first time)
[2026-10-08T21:00:20Z INFO  ferrite::fonts] CJK fonts state: KR=true, JP=true, SC=true, TC=false
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Loaded fonts: CJK(KR=true, JP=true, SC=true, TC=false), ComplexScript=false, customs=[]
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Font generation bumped to 3
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Font generation bumped to 4
[2026-10-08T21:00:20Z INFO  ferrite::fonts] Pre-warmed font atlas with 41 box-drawing, 43 symbol, and 9 emoji characters
```

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-230020-T2-32.log (also saved as logs/T2-32-fail-tail.txt; purely visual defect, no WARN/ERROR at the failure; clean exit 0)</summary>

```text
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
</details>


#### ISS-31: deadlock with Preview lock ON (T2-35; same stack as ISS-1)

Backtrace excerpt, `logs/T2-35-relaunch-hang-bt.txt`, main thread (second freeze: click on a blank area of the Rendered pane, then 1 ArrowDown):

```text
Thread 1 (Thread 0x7f9e01b10300 (LWP 473787) "ferrite"):
#0  0x00007f9e128d64f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000055f563650249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000055f5630c4b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000055f5630d1434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000055f562d270ae in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#5  0x000055f562e20e5c in egui::ui::Ui::scope_dyn ()
#6  0x000055f562cf1228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#7  0x000055f562e20e5c in egui::ui::Ui::scope_dyn ()
#8  0x000055f562cb1278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x000055f5630c0242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#10 0x000055f5630c2faa in ferrite::markdown::editor::MarkdownEditor::show ()
#11 0x000055f562fa1a90 in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
#12 0x000055f562df00ff in egui::containers::panel::CentralPanel::show_inside_dyn ()
#13 0x000055f562f7ae1d in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel ()
#14 0x000055f562f5a3f2 in ferrite::app::FerriteApp::render_ui ()
```

`logs/T2-35-hang-bt.txt` (first freeze) has the same main-thread frames #0–#11:

```text
Thread 1 (Thread 0x7f7686879300 (LWP 466680) "ferrite"):
#0  0x00007f769763f4f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000055c65c4b2249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000055c65bf26b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000055c65bf33434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000055c65bb890ae in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#5  0x000055c65bc82e5c in egui::ui::Ui::scope_dyn ()
#6  0x000055c65bb53228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#7  0x000055c65bc82e5c in egui::ui::Ui::scope_dyn ()
#8  0x000055c65bb13278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x000055c65bf22242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#10 0x000055c65bf24faa in ferrite::markdown::editor::MarkdownEditor::show ()
#11 0x000055c65be03a90 in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
```

<details><summary>Last 30 non-span lines, first freeze: logs/ferrite-debug-20261008-230821-T2-35.log (log goes silent after the switch to Rendered; kill -9, exit 137)</summary>

```text
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
</details>

<details><summary>Last 30 non-span lines, second freeze: logs/ferrite-debug-20261008-231259-T2-35-relaunch-a.log (kill -9, exit 137)</summary>

```text
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
</details>


#### ISS-32: Mermaid flowchart overlaps (T3-43)

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-234101-T3-43.log (purely visual defect, no WARN/ERROR or mermaid error lines; clean exit 0)</summary>

```text
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
</details>


#### ISS-33: adjacent single-line `<div align>` (T3-44)

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-234543-T3-44.log (purely visual defect, no WARN/ERROR; clean exit 0)</summary>

```text
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
</details>


#### ISS-34: forwarded open does not raise the window (SI-1)

The log's tail is from the later kill -9 in SI-5 (the same primary process was killed at 22:04:39Z, exit 137). The forward events that matter for SI-1 are at 21:57:13Z and 21:57:48Z (the retry); they follow the tail. Note the two `request_user_attention` lines and the absence of any focus/raise call.

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261008-235629-SI-1.log</summary>

```text
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
</details>

<details><summary>Forwarded-open events for SI-1: non-span lines 2731–2784 of the same log (21:57:12Z–21:57:48Z)</summary>

```text
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
</details>


### Log excerpts, Batches 7–12

Same method as above (`grep -v 'tracing::span' LOG | tail -n 30`, or targeted greps that drop the periodic recovery-snapshot and wrap-width lines). Lines over 300 chars are truncated.

#### ISS-1 (Batches 9–12 additions)

Ten more gdb dumps, all with the same main-thread frames #0–#3 (`syscall` ← `lock_shared_slow` ← `arrow_nav_sel_key` ← `remember_arrow_nav_sel`): `logs/ARR-1-hang-bt.txt`, `ARR-1b-hang-bt.txt`, `ARR-2-hang-bt.txt`, `ARR-2b-hang-bt.txt`, `ARR-4-hang-bt.txt`, `ARR-4b-hang-bt.txt`, `CPY-3-hang-bt.txt`, `IND-9-hang-bt.txt`, `UX-9-hang-bt.txt`, `UX-9-fresh-2-hang-bt.txt`. Two new callers are shown below.

<details><summary>UX-9 (plain click on a file link in a list item): logs/UX-9-hang-bt.txt, main thread</summary>

```text
Thread 1 (Thread 0x7efc6e3ce300 (LWP 971626) "ferrite"):
#0  0x00007efc7f1944f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000055a7eefad249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000055a7eea21b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000055a7eea2e434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000055a7eea36509 in ferrite::markdown::editor::render_session_formatted_edit_text ()
#5  0x000055a7ee66eb4e in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#6  0x000055a7eea10b1d in ferrite::markdown::editor::render_list ()
#7  0x000055a7eea112f9 in ferrite::markdown::editor::render_node ()
#8  0x000055a7ee6530a8 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x000055a7ee77d525 in egui::ui::Ui::allocate_ui_with_layout_dyn ()
#10 0x000055a7ee682ac7 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#11 0x000055a7ee77de5c in egui::ui::Ui::scope_dyn ()
#12 0x000055a7ee64e228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#13 0x000055a7ee77de5c in egui::ui::Ui::scope_dyn ()
#14 0x000055a7ee60e278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#15 0x000055a7eea1d242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#16 0x000055a7eea1ffaa in ferrite::markdown::editor::MarkdownEditor::show ()
#17 0x000055a7ee8fea90 in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
#18 0x000055a7ee74d0ff in egui::containers::panel::CentralPanel::show_inside_dyn ()
#19 0x000055a7ee8d7e1d in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel ()
#20 0x000055a7ee8b73f2 in ferrite::app::FerriteApp::render_ui ()
```
</details>

<details><summary>IND-9 (Tab on a list item in the Split preview): logs/IND-9-hang-bt.txt, main thread</summary>

```text
Thread 1 (Thread 0x7efbfc7ca300 (LWP 857568) "ferrite"):
#0  0x00007efc0d5904f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x000055a96374a249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x000055a9631beb6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x000055a9631cb434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x000055a9631d0552 in ferrite::markdown::editor::render_session_plain_text_block ()
#5  0x000055a962e0b0e3 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#6  0x000055a9631adb1d in ferrite::markdown::editor::render_list ()
#7  0x000055a9631ae2f9 in ferrite::markdown::editor::render_node ()
#8  0x000055a962df00a8 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x000055a962f1a525 in egui::ui::Ui::allocate_ui_with_layout_dyn ()
#10 0x000055a962e1fac7 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#11 0x000055a962f1ae5c in egui::ui::Ui::scope_dyn ()
#12 0x000055a962deb228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#13 0x000055a962f1ae5c in egui::ui::Ui::scope_dyn ()
#14 0x000055a962dab278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#15 0x000055a9631ba242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#16 0x000055a9631bcfaa in ferrite::markdown::editor::MarkdownEditor::show ()
#17 0x000055a96309d70b in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
#18 0x000055a962eea0ff in egui::containers::panel::CentralPanel::show_inside_dyn ()
#19 0x000055a963074e1d in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel ()
#20 0x000055a9630543f2 in ferrite::app::FerriteApp::render_ui ()
```
</details>

<details><summary>ARR-4 (ArrowDown with focus in the Split preview): logs/ARR-4-hang-bt.txt, main thread</summary>

```text
Thread 1 (Thread 0x7faeb7bd5300 (LWP 816108) "ferrite"):
#0  0x00007faec899b4f9 in syscall () at /lib/x86_64-linux-gnu/libc.so.6
#1  0x00005642880d3249 in parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ()
#2  0x0000564287b47b6b in ferrite::markdown::editor::arrow_nav_sel_key ()
#3  0x0000564287b54434 in ferrite::markdown::editor::remember_arrow_nav_sel ()
#4  0x00005642877aa0ae in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#5  0x00005642878a3e5c in egui::ui::Ui::scope_dyn ()
#6  0x0000564287774228 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#7  0x00005642878a3e5c in egui::ui::Ui::scope_dyn ()
#8  0x0000564287734278 in core::ops::function::FnOnce::call_once{{vtable.shim}} ()
#9  0x0000564287b43242 in ferrite::markdown::editor::MarkdownEditor::show_rendered_editor ()
#10 0x0000564287b45faa in ferrite::markdown::editor::MarkdownEditor::show ()
#11 0x0000564287a2670b in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}} ()
#12 0x00005642878730ff in egui::containers::panel::CentralPanel::show_inside_dyn ()
#13 0x00005642879fde1d in ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel ()
#14 0x00005642879dd3f2 in ferrite::app::FerriteApp::render_ui ()
#15 0x00005642878d1ab2 in <ferrite::app::FerriteApp as eframe::epi::App>::ui ()
#16 0x00005642875d7ca4 in egui::context::Context::run_ui_dyn::{{closure}} ()
#17 0x00005642875de8c5 in egui::context::Context::run_dyn ()
#18 0x00005642875d77a1 in egui::context::Context::run_ui_dyn ()
#19 0x00005642874ffd25 in <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint ()
#20 0x00005642875091f8 in <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event ()
```
</details>

<details><summary>UX-9 second freeze: logs/ferrite-debug-20261009-044244-UX-9-fresh.log, last 30 non-span lines (kill -9, exit 137)</summary>

```text
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:43:12Z DEBUG ferrite::editor::widget] Syncing 5 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:43:12Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1460.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:43:12Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 0, y: 235 }) size=Physical(PhysicalSize { width: 1500, height: 911 })
[2026-10-09T02:43:12Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-09T02:43:12Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-09T02:43:19Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:43:19Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:43:19Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:43:19Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T02:43:19Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 410, y: 449 }) size=Physical(PhysicalSize { width: 356, height: 17 })
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 04:45:53 CEST ===
```
</details>

#### ISS-2 (EXTRA-YT-7, Batch 12b)

<details><summary>EXTRA-YT-7 fresh instance: logs/ferrite-debug-20261009-050216-EXTRA-YT-7-fresh.log, last 30 non-span lines (exit 134)</summary>

```text
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 05:02:39 CEST ===
```
</details>

#### ISS-33 (HTML-6)

<details><summary>HTML-6: logs/batch12/HTML-6-tail.txt (as saved by the tester)</summary>

```text
[2026-10-09T02:19:54Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:19:55Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:19:55Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:19:55Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:19:55Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:19:55Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:19:55Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:19:55Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:20:01Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:20:01Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:20:01Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:20:01Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:20:01Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:20:01Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:20:01Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:20:06Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:20:06Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:20:06Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:20:06Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:20:06Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:20:06Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:20:06Z DEBUG ferrite::app] Crash recovery snapshot saved
```
</details>

#### ISS-36 and ISS-63: SES-6

<details><summary>SES-6: logs/ferrite-debug-20261009-024942-SES-6.log, last 30 non-span lines (exit 0)</summary>

```text
[2026-10-09T00:52:23Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T00:52:23Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:52:23Z INFO  ferrite::app] Application exiting
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 63 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 64 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 66 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 67 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 68 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 72 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saved recovery content for tab 73 (path=None, hash=None)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Deleted recovery content for tab 71
[2026-10-09T00:52:23Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=true
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 39 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:52:23Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:52:23Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:52:23Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T00:52:23Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:52:23Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:52:23Z INFO  ferrite::state] Settings saved
[2026-10-09T00:52:23Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:52:23Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:52:23Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:52:23Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:52:23Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:52:23Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:52:23Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:52:23 CEST ===
```
</details>

#### ISS-37: TTL-6b (Ctrl+, logged, Settings not opened)

<details><summary>Ctrl+, and settings lines: logs/ferrite-debug-20261009-050403-TTL-6b-on.log</summary>

```text
[2026-10-09T03:04:03Z DEBUG winit::event_loop] winit::ActiveEventLoop::create_window; window_attributes=WindowAttributes { inner_size: Some(Logical(LogicalSize { width: 1920.0, height: 1200.0 })), min_inner_size: Some(Logical(LogicalSize { width: 400.0, height: 300.0 })), max_inner_size: None, posit …[truncated]
[2026-10-09T03:04:03Z INFO  ferrite::state] AppState initialized with settings
[2026-10-09T03:04:03Z DEBUG ferrite::state] Restored tab 99 from session: Untitled
[2026-10-09T03:04:03Z DEBUG ferrite::workspaces::settings] No workspace settings file at "/workspace/ferrite-qa/ferrite/.ferrite/settings.json"
[2026-10-09T03:04:04Z INFO  ferrite::app] Loaded app logo texture for title bar
[2026-10-09T03:04:04Z DEBUG winit::window] winit::Window::set_title; title="test_video_embeds.md - Ferrite"
[2026-10-09T03:04:52Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+, (Open Settings)
[2026-10-09T03:05:32Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+, (Open Settings)
[2026-10-09T03:05:40Z DEBUG winit::window] winit::Window::set_title; title="\u{e2ce} About / Help - Ferrite"
[2026-10-09T03:06:01Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+, (Open Settings)
[2026-10-09T03:06:22Z INFO  ferrite::state] Settings saved
[2026-10-09T03:06:22Z INFO  ferrite::state] Settings saved
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-050403-TTL-6b-on.log</summary>

```text
[2026-10-09T03:06:22Z INFO  ferrite::state] Settings saved
[2026-10-09T03:06:22Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T03:06:22Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T03:06:22Z INFO  ferrite::app] Application exiting
[2026-10-09T03:06:22Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T03:06:22Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=53, clean_shutdown=true
[2026-10-09T03:06:22Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T03:06:22Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T03:06:22Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 53 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T03:06:22Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T03:06:22Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T03:06:22Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 1 expanded paths)
[2026-10-09T03:06:22Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T03:06:22Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T03:06:22Z INFO  ferrite::state] Settings saved
[2026-10-09T03:06:22Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T03:06:22Z DEBUG ferrite::terminal::pty] Terminal child process killed on drop
[2026-10-09T03:06:22Z DEBUG ferrite::terminal::pty] Terminal child exited with status: ExitStatus { code: 1, signal: Some("Hangup") }
[2026-10-09T03:06:22Z DEBUG ferrite::terminal::pty] TerminalPty dropped, resources released
[2026-10-09T03:06:22Z DEBUG ferrite::terminal::pty] PTY reader thread exiting
[2026-10-09T03:06:22Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T03:06:22Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T03:06:22Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T03:06:22Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T03:06:22Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T03:06:22Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T03:06:22Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T03:06:22Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T03:06:22Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 05:06:22 CEST ===
```
</details>

#### ISS-38: SES-1 (Exit → Save skips Untitled)

<details><summary>Save / exit / recovery lines: logs/ferrite-debug-20261009-024031-SES-1.log</summary>

```text
[2026-10-09T00:40:47Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:40:52Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:40:57Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:02Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:08Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:13Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:18Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:23Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:28Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=38, clean_shutdown=false
[2026-10-09T00:41:33Z DEBUG ferrite::state] Created new tab 69 in window 0
[2026-10-09T00:41:33Z DEBUG ferrite::editor::widget] Creating new FerriteEditor for tab 69
[2026-10-09T00:41:33Z DEBUG ferrite::editor::widget] EditorWidget: auto-focused tab 69
[2026-10-09T00:41:33Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:41:38Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:41:44Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:41:49Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:41:54Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:41:59Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:42:04Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:42:05Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-09T00:42:09Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:42:14Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=false
[2026-10-09T00:42:17Z INFO  ferrite::state] Saved file: /tmp/ses1-a.md (encoding: UTF-8)
[2026-10-09T00:42:17Z INFO  ferrite::state] Saved file: /tmp/ses1-b.md (encoding: UTF-8)
[2026-10-09T00:42:17Z DEBUG ferrite::state] Exit confirmed
[2026-10-09T00:42:17Z INFO  ferrite::app] Application exiting
[2026-10-09T00:42:17Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=39, clean_shutdown=true
[2026-10-09T00:42:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:42:17Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
=== Ferrite exited with code 0 clean exit at 02:42:17 CEST ===
```
</details>

#### ISS-39: CPY-2 (copy inserts spaces)

<details><summary>Clipboard as captured: logs/CPY-2-selection-clipboard.txt</summary>

```text
tems. iter(). m
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-031127-CPY-2.log</summary>

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
</details>

#### ISS-40: UND-3 (Rendered code-block typing / undo)

<details><summary>Undo and rendered-edit lines: logs/ferrite-debug-20261009-031831-UND-3.log</summary>

```text
[2026-10-09T01:18:31Z DEBUG ferrite::state] Restored tab: /workspace/ferrite-qa/ferrite/test_md/test_copy_and_split_undo.md (encoding: UTF-8)
[2026-10-09T01:18:31Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_copy_and_split_undo'
[2026-10-09T01:18:31Z DEBUG winit::window] winit::Window::set_title; title="test_copy_and_split_undo.md - Ferrite"
[2026-10-09T01:18:53Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_copy_and_split_undo.md
[2026-10-09T01:19:14Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:19:14Z DEBUG winit::window] winit::Window::set_title; title="test_copy_and_split_undo.md* - Ferrite"
[2026-10-09T01:19:14Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:19:58Z DEBUG ferrite::app::input_handling] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-10-09T01:19:58Z DEBUG ferrite::app::navigation] Undo performed, 1 entries remaining
[2026-10-09T01:20:06Z DEBUG ferrite::app::input_handling] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-10-09T01:20:06Z DEBUG ferrite::app::navigation] Undo performed, 0 entries remaining
[2026-10-09T01:20:06Z DEBUG winit::window] winit::Window::set_title; title="test_copy_and_split_undo.md - Ferrite"
[2026-10-09T01:20:25Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:25Z DEBUG winit::window] winit::Window::set_title; title="test_copy_and_split_undo.md* - Ferrite"
[2026-10-09T01:20:25Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:25Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:25Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:26Z DEBUG ferrite::app::central_panel] Content modified in rendered editor, recorded for undo
[2026-10-09T01:20:51Z DEBUG ferrite::app::input_handling] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-10-09T01:20:51Z DEBUG ferrite::app::navigation] Undo performed, 12 entries remaining
[2026-10-09T01:21:03Z DEBUG ferrite::app::input_handling] Keyboard shortcut: Ctrl+Z (Undo) - consumed before render
[2026-10-09T01:21:03Z DEBUG ferrite::app::navigation] Undo performed, 11 entries remaining
```
</details>

#### ISS-41: TBL-2 (cell edit committed late)

<details><summary>Save lines for `/tmp/tbl2.md`: logs/ferrite-debug-20261009-041820-WS-1.log</summary>

```text
[2026-10-09T02:33:32Z INFO  ferrite::state] Opened file: /tmp/tbl2.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T02:33:32Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /tmp/tbl2.md
[2026-10-09T02:33:32Z DEBUG winit::window] winit::Window::set_title; title="tbl2.md - Ferrite"
[2026-10-09T02:34:19Z DEBUG winit::window] winit::Window::set_title; title="tbl2.md* - Ferrite"
[2026-10-09T02:35:55Z INFO  ferrite::state] Saved file: /tmp/tbl2.md (encoding: UTF-8)
[2026-10-09T02:35:55Z DEBUG winit::window] winit::Window::set_title; title="tbl2.md - Ferrite"
[2026-10-09T02:36:11Z DEBUG winit::window] winit::Window::set_title; title="tbl2.md* - Ferrite"
[2026-10-09T02:36:50Z INFO  ferrite::state] Saved file: /tmp/tbl2.md (encoding: UTF-8)
[2026-10-09T02:36:50Z DEBUG winit::window] winit::Window::set_title; title="tbl2.md - Ferrite"
```
</details>

#### ISS-42: UX-9 (Ctrl+click / middle-click links)

<details><summary>Open / reveal lines (nothing is logged for the Ctrl+click and middle-click attempts; only the forwarded open of the fixture): logs/ferrite-debug-20261009-041820-WS-1.log</summary>

```text
[2026-10-09T02:40:34Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_local_links.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T02:40:34Z DEBUG ferrite::app::file_ops] Opened file from secondary instance: /workspace/ferrite-qa/ferrite/test_md/test_local_links.md
[2026-10-09T02:40:34Z DEBUG winit::window] winit::Window::set_title; title="test_local_links.md - Ferrite"
```
</details>

#### ISS-43: EXTRA-SET-5 (Auto-load Layout OFF ignored)

<details><summary>Layout / terminal lines: logs/ferrite-debug-20261009-051246-EXTRA-SET-5-relaunch.log</summary>

```text
[2026-10-09T03:13:18Z INFO  ferrite::ui::terminal_panel] Loaded workspace layout from "/workspace/ferrite-qa/ferrite/.ferrite/terminal-layout.json"
[2026-10-09T03:13:18Z INFO  ferrite::ui::terminal_panel] Terminal panel initialized from workspace layout
[2026-10-09T03:13:18Z DEBUG ferrite::app::navigation] Terminal panel toggled: true
```
</details>

#### ISS-44 and ISS-4 (SIF-3)

<details><summary>Search-in-Files / caret lines: logs/ferrite-debug-20261009-014003-SIF-3.log</summary>

```text
[2026-10-08T23:40:04Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 8) -> (0, 0)
[2026-10-08T23:40:19Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (0, 0) -> (2, 0)
[2026-10-08T23:40:53Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+F (Search in Files)
[2026-10-08T23:44:11Z DEBUG ferrite::app::file_ops] Opened file from search: /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md at line 2, char offset 16
[2026-10-08T23:44:11Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (1, 1) -> (1, 0)
[2026-10-08T23:44:43Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+F (Search in Files)
[2026-10-08T23:45:22Z DEBUG ferrite::app::file_ops] Opened file from search: /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md at line 2, char offset 15
[2026-10-08T23:45:40Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+F (Search in Files)
[2026-10-08T23:45:45Z DEBUG ferrite::app::file_ops] Opened file from search: /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md at line 2, char offset 33
[2026-10-08T23:45:45Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (1, 18) -> (1, 0)
[2026-10-08T23:46:11Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+F (Search in Files)
[2026-10-08T23:46:34Z DEBUG ferrite::app::file_ops] Opened file from search: /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md at line 2, char offset 24
[2026-10-08T23:46:34Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (1, 9) -> (1, 0)
[2026-10-08T23:47:32Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+Shift+F (Search in Files)
[2026-10-08T23:49:19Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (1, 0) -> (2, 0)
[2026-10-08T23:50:11Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (2, 2) -> (2, 0)
[2026-10-08T23:51:07Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (2, 1) -> (2, 0)
[2026-10-08T23:52:12Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (2, 1) -> (2, 0)
[2026-10-08T23:52:29Z DEBUG ferrite::editor::widget] EditorWidget: cursor sync changed: (2, 1) -> (2, 0)
```
</details>

#### ISS-45: PRI-3 (middle-click on tab)

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-015853-PRI-1.log (no tab-close line for the middle-clicks)</summary>

```text
[2026-10-09T00:02:17Z DEBUG eframe::native::glow_integration] Received WindowEvent::CloseRequested for viewport Some("FFFF")
[2026-10-09T00:02:17Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (2 recent files, 1 expanded paths)
[2026-10-09T00:02:17Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:02:17Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:02:17Z INFO  ferrite::state] Settings saved
[2026-10-09T00:02:17Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:02:17Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:02:17Z INFO  ferrite::app] Application exiting
[2026-10-09T00:02:17Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:02:17Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=29, clean_shutdown=true
[2026-10-09T00:02:17Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:02:17Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:02:17Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 29 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:02:17Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:02:17Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:02:17Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (2 recent files, 1 expanded paths)
[2026-10-09T00:02:17Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:02:17Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:02:17Z INFO  ferrite::state] Settings saved
[2026-10-09T00:02:17Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:02:17Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:02:17Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:02:17Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:02:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:02:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:02:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:02:17Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:02:17Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:02:17Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:02:17 CEST ===
```
</details>

#### ISS-46 and ISS-5: IMG-X1 / IMG-X2 (silent paste failure)

<details><summary>arboard lines: logs/ferrite-debug-20261009-020219-IMG-X1.log</summary>

```text
[2026-10-09T00:02:19Z DEBUG egui_winit::clipboard] Cannot init smithay clipboard without a Wayland display handle
[2026-10-09T00:02:19Z DEBUG ferrite::state] Restored tab: /workspace/ferrite-qa/ferrite/test_md/test_local_images.md (encoding: UTF-8)
[2026-10-09T00:02:19Z DEBUG ferrite::state] Restored image viewer tab: /workspace/ferrite-qa/ferrite/assets/icons/icon-192x192.png
[2026-10-09T00:02:19Z DEBUG ferrite::state] Restored tab 39 from session: test_local_images.md
[2026-10-09T00:02:52Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-09T00:03:06Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
```
</details>

<details><summary>arboard lines: logs/ferrite-debug-20261009-020435-IMG-X2.log</summary>

```text
[2026-10-09T00:04:35Z DEBUG egui_winit::clipboard] Cannot init smithay clipboard without a Wayland display handle
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored tab: /workspace/ferrite-qa/ferrite/test_md/test_local_images.md (encoding: UTF-8)
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored image viewer tab: /workspace/ferrite-qa/ferrite/assets/icons/icon-192x192.png
[2026-10-09T00:04:35Z DEBUG ferrite::state] Restored tab 39 from session: test_local_images.md
[2026-10-09T00:04:57Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-09T00:04:57Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
[2026-10-09T00:05:16Z ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard
```
</details>

#### ISS-47 and ISS-48: spellcheck (SPL-2, SPL-5, SPL-7)

<details><summary>Spellcheck lines: logs/ferrite-debug-20261009-020610-SPL-1.log (none are logged)</summary>

```text
(no matching lines)
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-020610-SPL-1.log</summary>

```text
[2026-10-09T00:17:16Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::state] Settings saved
[2026-10-09T00:17:16Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:17:16Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T00:17:16Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T00:17:16Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1202.4, max_wrap_width=840.0, effective=840.0
[2026-10-09T00:17:16Z DEBUG eframe::native::epi_integration] Closing root viewport (ViewportCommand::CancelClose was not sent)
[2026-10-09T00:17:16Z INFO  ferrite::app] Application exiting
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saved recovery content for tab 57 (path=None, hash=None)
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saved recovery content for tab 58 (path=None, hash=None)
[2026-10-09T00:17:16Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Saving session state: is_recovery=false, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=31, clean_shutdown=true
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.tmp
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.json
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.json (1 windows, 31 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T00:17:16Z INFO  ferrite::app] Session state saved for next startup
[2026-10-09T00:17:16Z DEBUG ferrite::config::session] Removed session lock file
[2026-10-09T00:17:16Z DEBUG ferrite::workspaces::persistence] Saved workspace state to "/workspace/ferrite-qa/ferrite/.ferrite/state.json" (3 recent files, 2 expanded paths)
[2026-10-09T00:17:16Z DEBUG ferrite::config::persistence] Saving config to: /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::config::persistence] Configuration saved successfully to /home/box/.config/ferrite/config.json
[2026-10-09T00:17:16Z INFO  ferrite::state] Settings saved
[2026-10-09T00:17:16Z INFO  ferrite::state] AppState shutdown complete
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance lock file
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance port file
[2026-10-09T00:17:16Z DEBUG ferrite::single_instance] Cleaning up instance pid file
[2026-10-09T00:17:16Z DEBUG eframe::native::run] Asking to exit event loop…
[2026-10-09T00:17:16Z DEBUG eframe::native::run] Received Event::LoopExiting - saving app state…
[2026-10-09T00:17:16Z DEBUG eframe::native::run] eframe window closed
=== Ferrite exited with code 0 clean exit at 02:17:16 CEST ===
```
</details>

#### ISS-49: STAT-3

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-023201-STAT-3.log (no list-continuation lines)</summary>

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
</details>

#### ISS-50: EOL-9

<details><summary>Checker output: logs/EOL-9-check.txt</summary>

```text
file: /tmp/mixed-eol.md: ASCII text, with CRLF, LF line terminators
lines ending in CR: 4 / total lines: 5
RESULT: NOT all CRLF (False)
00000000: 2320 4d69 7865 640d 0a0d 0a6c 696e 6520  # Mixed....line 
00000010: 6372 6c66 0d0a 6c69 6e65 206c 660a 656e  crlf..line lf.en
00000020: 6420 780d 0a                             d x..
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-023851-EOL-9.log</summary>

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
</details>

#### ISS-51: IND-2

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-032305-RLD-3.log (same instance as IND-1…8; no panic)</summary>

```text
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:42Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:42Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:43Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:43Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
[2026-10-09T01:40:44Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T01:40:44Z DEBUG ferrite::editor::widget] Syncing 8 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T01:40:44Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=706.3, max_wrap_width=840.0, effective=706.3
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 03:42:17 CEST ===
```
</details>

#### ISS-52, ISS-53, ISS-61, ISS-62: WS-1 instance (UX-8, UX-11, TBL-3, Alt+Z)

<details><summary>Alt+Z / word-wrap lines: logs/ferrite-debug-20261009-041820-WS-1.log</summary>

```text
[2026-10-09T02:30:53Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Alt+Z (Toggle Word Wrap)
[2026-10-09T02:30:53Z INFO  ferrite::app::navigation] Word wrap disabled
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-041820-WS-1.log</summary>

```text
[2026-10-09T02:40:43Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 535, y: 475 }) size=Physical(PhysicalSize { width: 356, height: 17 })
[2026-10-09T02:40:43Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:43Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:48Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:48Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:53Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:53Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:58Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:58Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 04:42:16 CEST ===
```
</details>

#### ISS-54: TTL-6a

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-045325-GIT-2.log (no window-geometry lines were logged for the title-bar actions; this process later aborted in EXTRA-YT-7)</summary>

```text
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 05:01:39 CEST ===
```
</details>

#### ISS-55: RUN-1

<details><summary>Matching lines: logs/ferrite-debug-20261009-044740-RUN-1.log (Ferrite logs nothing about code runs, Stop or timeouts; only startup and exit)</summary>

```text
args: --log-level debug /workspace/ferrite-qa/ferrite/test_md/test_code_execution.md
[2026-10-09T02:47:40Z INFO  ferrite] CLI paths provided: ["/workspace/ferrite-qa/ferrite/test_md/test_code_execution.md"]
[2026-10-09T02:47:41Z DEBUG ferrite::state] Restored tab: /workspace/ferrite-qa/ferrite/test_md/test_code_execution.md (encoding: UTF-8)
[2026-10-09T02:47:41Z DEBUG ferrite::state] Restored tab 62 from session: test_code_execution.md
[2026-10-09T02:47:41Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_code_execution.md
[2026-10-09T02:47:41Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_code_execution'
[2026-10-09T02:47:41Z DEBUG winit::window] winit::Window::set_title; title="test_code_execution.md - Ferrite"
[2026-10-09T02:53:02Z WARN  arboard::platform::linux::x11] Could not hand the clipboard contents over to the clipboard manager. The request timed out.
=== Ferrite exited with code 0 clean exit at 04:53:02 CEST ===
```
</details>

#### ISS-56: EXTRA-SET-7

<details><summary>Language / font lines: logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log</summary>

```text
[2026-10-09T03:23:55Z INFO  ferrite] Language: English (en)
[2026-10-09T03:23:56Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T03:23:57Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
[2026-10-09T03:25:32Z INFO  ferrite::app::central_panel] Loaded CJK fonts for language: ChineseSimplified
[2026-10-09T03:26:07Z INFO  ferrite::app::central_panel] Loaded CJK fonts for language: Japanese
[2026-10-09T03:26:07Z WARN  epaint::text::fonts] Failed to find replacement characters '◻' or '?'. Will use empty glyph.
```
</details>

#### ISS-57, ISS-58, ISS-59, ISS-60: Mermaid (Batch 11)

<details><summary>Mermaid lines: logs/ferrite-debug-20261009-035014-MMD-1.log</summary>

```text
args: --log-level debug /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md
[2026-10-09T01:50:14Z INFO  ferrite] CLI paths provided: ["/workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md", "/workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md"]
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_v031.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
[2026-10-09T01:50:14Z INFO  ferrite::app] Opening file from CLI: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md
[2026-10-09T01:50:14Z INFO  ferrite::state] Opened file: /workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_83.md (encoding: UTF-8, auto-save: false, view_mode: Raw)
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
[2026-10-09T02:02:36Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
[2026-10-09T02:08:01Z DEBUG ferrite::app::navigation] Backlinks (directory scan): 0 backlinks for 'test_mermaid_issue_165'
[2026-10-09T02:08:01Z DEBUG winit::window] winit::Window::set_title; title="test_mermaid_issue_165.md - Ferrite"
[2026-10-09T02:14:54Z DEBUG ferrite::ui::outline_panel] Outline: clicked heading 'FC-165b — Auth sequence diagram (autonumber)' at line 88
[2026-10-09T02:14:54Z DEBUG ferrite::app::navigation] Navigated to heading 'FC-165b — Auth sequence diagram (autonumber)' at line 88 (char range: Some((3785, 3834)))
```
</details>

<details><summary>Last 30 non-span lines: logs/ferrite-debug-20261009-040706-MMD-10-control.log</summary>

```text
=== Ferrite QA launch 2026-10-09 04:07:06 CEST ===
binary: /workspace/ferrite-qa/ferrite/target/release/ferrite
version: ferrite 0.3.1
commit: 8e6ef508aef141b243b961e278861d76f8bc0d48
DISPLAY=:2 WAYLAND_DISPLAY= LIBGL_ALWAYS_SOFTWARE= WINIT_UNIX_BACKEND= FERRITE_DIAG=
cwd: /workspace/ferrite-qa
args: --log-level debug /tmp/qa11/mmd10-autonumber2.md
===
=== Ferrite exited with code 0 clean exit at 04:07:06 CEST ===
```
</details>

The other issues from Batches 1–6 (ISS-13 to ISS-29, and ISS-35) are minor or polish. Their evidence is the screenshot or log named in the table. ISS-36 to ISS-63 have excerpts in the Batches 7–12 section above. The full debug logs are in `logs/` (inside the `qa-logs-*.zip` attachments).


## Not tested and why
All planned batches (0–12) have now run. Every row in the brief's lists has a verdict in this report or is listed here. Rows or sub-steps that still have no real verdict:

- **Skipped by the platform rule (brief §5):** row 40, TTL-1…TTL-5, MAC-1…MAC-4, PLT-1, PLT-3, RUN-2 `(Windows)`, GIT-1 (Windows `\\?\` paths; GIT-3 confirmed no `\\?\` text on Linux). Counted as SKIPPED (platform).
- **Skipped or not run for other reasons:**
  - MW-5: SKIPPED. No v1 session fixture exists in the repo (only live v2 session files), and the plan forbids hand-crafting one.
  - IMG-2: SKIPPED. Synthetic drags from Thunar never started a drag-and-drop session.
  - STAT-4 (optional): NOT RUN. Needs a German keyboard layout with dead keys.
- **Blocked by ISS-2 (GDK crash on any trusted YouTube embed):** VID-2, VID-3, VID-6…VID-9, VID-§9…§11; EXTRA-YT-1 cases A2–A14, C5–C9, D1–D8; EXTRA-YT-3, -5, -8; the players / Settings overlay / reload dialog / Search-in-Files steps of EXTRA-YT-7; the drag-resize half of EOL-7; the play/overlay/resize steps of row 36 and the play-video sub-step of row 37. VID-10 and PLT-2 have derived verdicts only.
- **Blocked by ISS-1 (deadlock):** the remaining checks of rows 2, 17, 18, 18b, 22 and 35, and the parts of ARR-1/2/4, CPY-3, IND-9 and UX-9 after the freeze (e.g. UX-9 ignored link schemes).
- **Blocked by ISS-5 (Ctrl+V image paste on X11):** IMG-4 (undo of a pasted image) and IMG-7 (preview-lock gating of a paste).
- **Blocked or partial because of the test environment / synthetic input:**
  - Row 20 (Alt+click, xfwm4 `easy_click`); command palette (Alt+Space, xfwm4).
  - Row 34 and CPY-2 full-line selection (drags), SPL-3 suggestion click (tooltip over the context menu), MMD-1 hand-pan, TTL-6a edge-resize, TTL-6b drag/resize on native decorations (not recorded), GIT-3 refocus (the window never lost focus).
  - EXTRA-SET-6 Ctrl+Alt+Shift capture (Alt lost), row 39 and ISS-62 Alt+Z.
- **Partial for fixture or scope reasons:**
  - SPL-4: the *Add word* click itself was not pinned down; persistence was verified.
  - FNT-4: dash `printf` doesn't expand `\u`; octal/python equivalents used.
  - CSV-5: the named fixtures have no long values; scratch files were used.
  - RUN-1: the fixture's 5 s timeout did not apply (30 s default used); why was not investigated.
  - EXTRA-SET-5: only the upper extremes were tried; EXTRA-SET-7: cs, et, nb_NO, pt, ru are not offered (ISS-56).
  - TBL-2: whether a pending cell edit is lost on save + quit was not tested (ISS-41).
  - ISS-36 (SES-6) was seen once; its cause and reproducibility are not established.
- **Real-keyboard re-checks still recommended:** ISS-7, ISS-18, ISS-62, EXTRA-SET-6 Ctrl+Alt+Shift. (The Ctrl+, part of ISS-12 is now explained by ISS-37.)
- **Artifacts:** the older attachments (`qa-shots-1…3.zip`, `qa-logs.zip`) were deleted and rebuilt with Python `zipfile` (ZIP_DEFLATED, level 9), split by compressed size so that each zip is under 24,000,000 bytes: `qa-shots-1.zip` (23,277,469 bytes, 100 files: `shots/ARR-1-fail.png` … `shots/MMD-4-pass.png`); `qa-shots-2.zip` (23,361,334 bytes, 131 files: `shots/MMD-4-popup-100.png` … `shots/T1-20-fail.png`); `qa-shots-3.zip` (23,292,490 bytes, 49 files: `shots/T1-20-probe.png` … `shots/T2-30-split-f3-wrap.png`); `qa-shots-4.zip` (23,290,511 bytes, 70 files: `shots/T2-30-split-final.png` … `shots/T3-41-partial-rendered-alignment.png`); `qa-shots-5.zip` (12,462,959 bytes, 56 files: `shots/T3-41-raw-check-tmp.png` … `shots/set-persist-recent.png`); `qa-logs-1.zip` (23,270,207 bytes, 220 files: `logs/ARR-1-hang-bt.txt` … `logs/launcher-PRI-1.out`); `qa-logs-2.zip` (4,035,644 bytes, 40 files: `logs/launcher-RLD-3.out` … `qa-notes/report-format.md`). Together they hold every file in `shots/` (406) and `logs/` (246, including `logs/batch12/HTML-6-tail.txt`) exactly once (checked with `zipfile` name lists), plus `qa-notes/` copies of `results-batch1.md` … `results-batch12.md`, `gui-test-plan.md` and `report-format.md` in the last logs zip. No log had to be truncated: the largest, `logs/ferrite-debug-20261009-032305-RLD-3.log` (25,671,290 bytes raw), compresses well below the limit. Nothing was committed, pushed or filed.

## Tag-gate opinion

No. I would not tag v0.3.1 from commit `8e6ef508` on the Linux evidence. Tier 1 is still 13 PASS / 11 FAIL / 1 BLOCKED of 25, and three must-pass rows (4b, 15, 21) fail. Over all 198 counted rows the result is 83 PASS / 51 FAIL / 30 PARTIAL / 17 BLOCKED / 16 SKIPPED / 1 NOT RUN.

Top blockers, in order:
1. **ISS-1, Rendered/Split self-deadlock.** This is the biggest risk in the release. Batches 9–12 showed it is not an edge case: plain ArrowUp/ArrowDown in Rendered or in the Split preview, Tab on a list item, a drag-select, and a single click on a link in a list all freeze the UI thread. 22 freezes, 19 gdb dumps, one stack (`arrow_nav_sel_key` reading `ui.memory()` inside a `ui.memory_mut()` closure). Nothing in that path is Linux-specific, so I would expect Windows to be affected too.
2. **ISS-2, YouTube embed crash on Linux.** Any trusted embed aborts the process (`GDK has not been initialized`), 9 launches. It also blocks most of the video rows.
3. **ISS-36, possible data loss after a session restore.** Seen once, cause unknown, but typing into a restored tab replaced the entire buffer. It needs a reproduction attempt before tagging.
4. **Must-pass Tier 1 failures:** UTF-16 classified as binary (ISS-3), Search-in-Files caret reset (ISS-4, also for ASCII), backtick auto-close while typing a fence (ISS-7, needs a real-keyboard check); then Ctrl+V image paste on X11 (ISS-5), split-view undo (ISS-6) and the Mermaid popup body click (ISS-8, partly working per Batch 11).

High (major) items to fix or consciously accept: ISS-37 (system title bar leaves no way into Settings; Ctrl+, is a no-op), ISS-38 (Exit → Save skips Untitled tabs), ISS-41 (Rendered/preview edits commit late, so a save can miss a pending cell edit), ISS-39 (copy from a code block inserts spaces), ISS-40 (Rendered code-block typing/undo), ISS-42 (Ctrl+click/middle-click links do nothing), ISS-43 (terminal Auto-load Layout OFF ignored), ISS-9 (corrupt config silently resets everything), ISS-10 (invisible text with the ANSI theme), and the rendering-fidelity majors ISS-30, ISS-32, ISS-33.

Suggested order: fix the deadlock (compute the memory key before taking the write lock), initialise GTK before creating the WebView on Linux with a thumbnail fallback that never panics, chase ISS-36, then the must-pass rows. After that, re-run Tier 1 on Linux and Windows with a real keyboard, plus the rows currently blocked by ISS-1, ISS-2 and ISS-5. The many minor findings from Batches 7–12 (ISS-44…ISS-63) are not tag-blocking on their own.
