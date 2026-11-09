# Ferrite v0.3.1 QA report — Linux

- Commit tested: `8e6ef508aef141b243b961e278861d76f8bc0d48` (`0.3.1-experimental`; commit date 2026-10-08 19:48 CEST). Binary reports `ferrite 0.3.1`; About shows "Version 0.3.1".
- Date / timezone: Thursday 2026-10-08, GUI testing 20:12–22:44 CEST (UTC+2). Times in this report are CEST unless they end in `Z`. Debug-log timestamps are UTC (`…Z`): CEST = Z + 2 h.
- Tier 2/3 follow-up: Batches 5–6 (Tier 2 rows 30–35 and 37–40, Tier 3 rows 41–47, §6 SI-1…5) were run Oct 8–9 2026, 22:55 CEST (Oct 8) to 00:06 CEST (Oct 9), on the same commit, binary and box. Raw results: `results-batch5.md`, `results-batch6.md`.
- Distro, kernel, display server (X11/Wayland), GPU/software GL: Debian GNU/Linux 13 (trixie), kernel `6.12.94+`. **X11 only**: Xvfb (`xvfb 2:21.1.16-1.3+deb13u4`) display `:2`, 1920×1200×24, xfwm4 4.20.0, no compositor, no Wayland. **No GPU**: Mesa 25.0.7 **llvmpipe** (LLVM 19.1.7, 256 bits) software GL, OpenGL 4.5 core, eframe glow renderer. WebView stack: WebKitGTK 4.1 **2.54.0** (`libwebkit2gtk-4.1-0 2.54.0-1~deb13u2`) via wry.
- rustc version: `rustc 1.92.0 (ded5c06cf 2025-12-08)`, pinned by the repo's `rust-toolchain.toml` (`channel = "1.92.0"`).
- Build time (release): **273 s** (19:56:30 → 20:01 CEST; cargo: "Finished `release` profile [optimized] target(s) in 4m 33s").
- Tester: Grok Bot (autonomous agent) driving the GUI with synthetic X11 mouse and keyboard input. Every launch went through `run-ferrite.sh`, which runs `--log-level debug` and records the exit code (`0` clean, `134` panic/abort, `137` kill -9).
- Repo state at the end (`git -C /workspace/ferrite-qa/ferrite status --short`, also saved to `logs/git-status-after-qa.txt`):
  ```
  ?? test_md/zz_cjk_search.md
  ```
  This is the untracked QA fixture created for row 15/SIF. It was left in place. No tracked file was modified, and nothing was committed, pushed, or filed as an issue.

**Overall: NOT ready to tag.** Tier 1: 13 PASS / 11 FAIL / 1 BLOCKED out of 25 rows. Two independent blockers:
(1) a main-thread self-deadlock in the Rendered/Split edit session (9 hang backtraces, all with the same stack: 7 from Tier 1, 2 more from T2-35);
(2) any trusted YouTube embed aborts the app on Linux (`GDK has not been initialized`).
Three of the six "must now pass" rows fail: 4b, 15, and 21. Row 38 was run in the Tier 2/3 follow-up and passes, relative path included.
Tier 2: 12 PASS / 3 FAIL / 2 PARTIAL / 1 BLOCKED / 1 SKIPPED out of 19. Tier 3: 3 PASS / 2 FAIL / 2 PARTIAL out of 7. The follow-up found no new crash. Its only hang (T2-35) is the known deadlock; the other new FAILs are rendering-fidelity issues (emoji, Mermaid subgraph layout, single-line `<div align>`) and a window-raise gap in single-instance forwarding (SI-1).

| Tier / set | Rows | PASS | FAIL | PARTIAL | BLOCKED | SKIPPED | NOT RUN |
|---|---|---|---|---|---|---|---|
| Minimal Tier 1 (1–22 incl. 4b, 16b, 18b) | 25 | 13 | 11 | 0 | 1 | 0 | 0 |
| Minimal Tier 2 (23–40 incl. 25b) | 19 | 12 | 3 | 2 | 1 | 1 (row 40, platform) | 0 |
| Minimal Tier 3 (41–47) | 7 | 3 | 2 | 2 | 0 | 0 | 0 |
| Total test, rows run directly (VID-1…9, VID-§9/§10/§11, EOL-7) | 13 | 1 | 1 | 2 | 9 | 0 | 0 |
| EXTRA-YT-1…8 (gui-test-plan) | 8 | 0 | 3 | 1 | 3 | 0 | 1 |
| EXTRA-SET-1…14 + EXTRA-SET-2-defect (gui-test-plan) | 15 | 5 | 2 | 3 | 0 | 0 | 5 |
| §6 extra rows (SIF, SI, PRI, SPL, FNT, IMG-X) | 27 | 4 | 1 | 0 | 0 | 0 | 22 |
| **All rows above** | **114** | **38** | **23** | **10** | **14** | **1** | **28** |

Total-test rows that are covered by a minimal or §6 row (e.g. FIND-1…4 → row 30) are listed in their own table below and are not counted again here. Row 39 is counted as PARTIAL (its Alt+Z sub-step is BLOCKED; everything else passed). Row 37 is counted as PASS; only its play-video sub-step was blocked by ISS-2.

### Environment caveats (read before triaging)

- **Software GL on a virtual framebuffer.** All rendering went through Mesa llvmpipe on Xvfb, with no GPU, no compositor, and no physical display. Some rendering findings may be partly environmental, especially the repaint lag where the UI updates only after the pointer moves (ISS-24) and any timing-sensitive behaviour. The deadlock (ISS-1) and the GDK panic (ISS-2) are **not** environmental. Both have deterministic stacks inside Ferrite code.
- **xfwm4 grabs Alt+click and Alt+Space.** `xfconf-query -c xfwm4 -p /general/easy_click` = `Alt`, so Alt+click never reached Ferrite. That blocked row 20. Alt+Space opens the xfwm4 window menu, so the command palette (default Alt+Space) was never reached. Alt+F4 worked. The WM setting was not changed.
- **Synthetic input.** Keys and clicks were injected as synthetic X events (xdotool-style) by an automated tester, not typed on a real keyboard. Fast injected key sequences may be dropped or reordered differently from real typing. This may have contributed to:
  - ISS-7: the auto-close backtick behaviour while typing a fence character by character. It reproduced 3 times, and the log shows Ferrite's own auto-close inserting a backtick after each keystroke.
  - ISS-18: the first keystroke dropped in Settings search or shortcut capture.
  - ISS-12: Ctrl+, not opening Settings.
  Please re-check these three with a real keyboard before acting on them.
  Batches 5–6 add two input gaps that are recorded as BLOCKED or PARTIAL, not FAIL: the mouse drag-select in row 34, and Alt+Z in row 39 (the synthetic Alt+Z arrived as a plain `z`). Ctrl+, again did not open Settings (SI-4).
- **Shared config directory.** `~/.config/ferrite/` held state across launches: session, recovery files, config. Hangs had to be cleared with `kill -9`, so several later launches opened with a Session Recovery dialog ("Start Fresh" was usually chosen). Before the first GUI launch, `~/.config/ferrite` already existed because `cargo test` had written to it (ISS-28). It was moved aside to `~/.config/ferrite.bak-1791483152`.
- **Network.** Network access was available; logs show `img.youtube.com` reached through `198.18.0.1`. The offline test (EXTRA-YT-2) used `sudo unshare -n`.
- **Hang diagnostics.** Hangs were diagnosed with `gdb -p <pid> -batch "thread apply all bt 40"` (`hang-dump.sh`) against the release binary. Function names resolve, but inlined frames are elided.
- **Screenshots.** Some rows have only in-session screenshots that were not saved as files: T1-19, EXTRA-SET-1, -4, -8, -11, and part of T2-24. They are listed as "none" in Evidence.

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

### Rows run directly (Batch 4: video + EOL-7)
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
| VID-10 | not recorded as a separate row | — | The `(Linux)` "builds with webkit2gtk **or degrades to thumbnail; no crash**" condition: the build compiled against WebKitGTK 4.1 (gate PASS), but the "no crash" half is contradicted by ISS-2. No separate verdict was recorded. |
| PLT-2 | not recorded as a separate row | `logs/build-release.log` | The release build succeeded with the WebView deps installed. The plan's PASS condition ("PASS if VID-1 plays") was not met because VID-1 crashed. |
| EOL-7 | PARTIAL | logs/EOL-7-check.txt; `…-223753-EOL-7.log` | Raw only, because Rendered would abort: x + Backspace + Ctrl+S → CRLF True 5/5, `0d0a`. The drag-resize part, which is the actual subject of the row, was **not** performed (needs Rendered). |

### EXTRA rows from gui-test-plan.md (Ola's focus areas: Settings, YouTube)
| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| EXTRA-YT-1 | PARTIAL | shots/VID-4-cases.png, shots/VID-4-pass.png; `…-223520-VID-1.log`, `…-223312-VID-4.log` | A1 (`watch?v=`) **FAIL (crash)**. A2–A14, C5–C9, D1–D8 BLOCKED (trusted URLs → crash). **B1–B5** (nocookie, no id, playlist-only, `youtube.com.evil.example`, vimeo) → placeholder, **no WebView: PASS**. **C1–C4** (`javascript:`, `file:///`, `notaurl`, `{{video}}`) → literal text, no embed: PASS (C1/C4 also show a grey hatched bar). |
| EXTRA-YT-2 | **FAIL (crash)** | shots/EXTRA-YT-2-fail-offline-crash.png; `…-224251-EXTRA-YT-2.log` | Offline (`unshare -n`). Rendered aborted (exit 134, 22:43:28). The thumbnail fetch failed with DNS error and was logged as WARN; 1 thumbnail-fetch WARN, no `ureq::` lines, no retry storm before the abort. The placeholder, error-on-play, and retry-storm checks were unreachable. |
| EXTRA-YT-3 | BLOCKED (crash) | — | No player ever rendered. |
| EXTRA-YT-4 | **FAIL (crash)** | shots/EXTRA-YT-4-fail-crash.png; `…-223923-EXTRA-YT-4.log` | `yt-many.md` (12 embeds): RSS 207,088 KB in Raw. Rendered aborted (exit 134, 22:39:37). |
| EXTRA-YT-5 | BLOCKED (crash) | — | Entering Rendered aborts. |
| EXTRA-YT-6 | **FAIL (crash)** | shots/EXTRA-YT-6-live1.png, shots/EXTRA-YT-6-live2.png, shots/EXTRA-YT-6-fail-crash.png; `…-223957-EXTRA-YT-6.log` | In Split, typing `{{video https://youtu.be/dQw4w9WgXcQ width=320}}` character by character aborted (exit 134, 22:42:22) on the keystroke that completed a trusted embed. The vimeo placeholder in the same doc was fine. |
| EXTRA-YT-7 | NOT RUN | — | Needs players. |
| EXTRA-YT-8 | BLOCKED (crash) | — | Needs a visible player. |
| TTL-6 | NOT RUN | — | Budget (Linux row in scope). The custom title bar was used all session. Incidental (batch 4, unconfirmed): a title-bar click once left the window in move mode, and a click on a top-right title-bar button toggled Zen Mode instead of closing. Not investigated. |
| EXTRA-SET-1 | PARTIAL | none saved; `…-221729-EXTRA-SET-10.log` | Ribbon gear opens Settings, and clicking the gear again *closes* it (toggle). Tab × closes it. Switching tabs keeps it, and there were no duplicate tabs. **Esc does not close Settings** (1 attempt). **Ctrl+, did not open Settings** (several attempts across rows; possibly input injection). Command palette not reachable (Alt+Space = WM menu; Ctrl+Shift+P = Export PDF). ISS-12. |
| EXTRA-SET-2 | PASS (persistence) | shots/SET-2-pass-before-quit.png, shots/EXTRA-SET-2-pass-mocha-visible.png; `…-215656-set-extra.log`, `…-220315-EXTRA-SET-2-relaunch.log` | Theme, accent, font, rendered font, size 20, syntax theme, scripts, and CJK preference all persisted and matched config.json. config.json is written only on exit. After relaunch, Recently Changed lists "Ferrite accent". |
| EXTRA-SET-2-defect | **FAIL** | shots/EXTRA-SET-2-fail-ansi-invisible-text.png; `…-220315-EXTRA-SET-2-relaunch.log` | ISS-10. Dark theme + Syntax Theme **ANSI** (+ JetBrains Mono 20px): Raw editor body text is invisible (reproduced 2×). Switching to Base16 Mocha Dark fixes it. |
| EXTRA-SET-3 | PASS (persistence) | shots/EXTRA-SET-3-pass.png; `…-215656-set-extra.log`, `…-220823-EXTRA-SET-3-relaunch.log` | 19 Editor values flipped and persisted. Effects verified: line numbers hidden, syntax highlighting off, minimap shown, Vim `[NORMAL]` on/off. Word-wrap effect not verified. Checkbox repaint lag (ISS-24); stale `[NORMAL]` on the Settings tab (ISS-25). |
| EXTRA-SET-4 | **FAIL** (final, after restart) | shots/set-persist-recent.png; `…-222853-set-persist.log` | Restore session OFF, welcome OFF, quick note OFF, and Auto-Save ON 15 s all persisted (log: "Session restore disabled in settings"). **Recent Files 0 was not honoured**: config.json had `max_recent_files=0` before launch, but the slider read "Remember last 10 files" after relaunch. ISS-11. |
| EXTRA-SET-5 | NOT RUN | — | Terminal persistence. Budget. |
| EXTRA-SET-6 | NOT RUN | — | Keyboard persistence. Budget. |
| EXTRA-SET-7 | NOT RUN | — | Language switch. Budget. |
| EXTRA-SET-8 | PARTIAL | none saved; `…-220823-EXTRA-SET-3-relaunch.log` | Max Line Width clamps correctly: 0 → 400, -5 → 400, 99999 → 2000, `abc` ignored. **No validation or feedback** for startup command (`;;;`, `$(rm -rf nowhere)` stored as text only, never executed), invalid regex `([`, or sound path `/nonexistent.wav` (ISS-19). Font fields are dropdowns and Tab Size is a slider, so they can't be typed into. LSP path hidden; no code-exec working-dir field found. No crash. |
| EXTRA-SET-9 | PASS (row criteria: no crash/hang) — but see ISS-9 | shots/EXTRA-SET-9a-pass.png; `…-221551-EXTRA-SET-9a.log`, `…-221645-EXTRA-SET-9b.log` | (a) `{ not json` and (b) `font_size: "abc"`: the app starts with **all defaults**, shows only a WARN in the log, and offers no toast or `.bak` restore. On clean exit it **overwrites config.json with defaults**. One bad field discards the whole config. |
| EXTRA-SET-10 | PASS | shots/EXTRA-SET-10-pass.png; `…-221729-EXTRA-SET-10.log` | `[*`, `\`, `.`, `中文`, 🙂, ` zoom `, `ZOOM`, and a 200-char query: sensible results, case-insensitive, no freeze. The 🙂 glyph is invisible in the field (ISS-29). The first char was dropped once (ISS-18). |
| EXTRA-SET-11 | PASS (after restart) | shots/set-persist-recent.png; `…-221729-EXTRA-SET-10.log`, `…-222853-set-persist.log` | Theme toggled 13×, minimap 11×, font slider dragged back and forth. No crash. Final values (Light, 72px, minimap OFF) persisted. Terminal opacity not tested. |
| EXTRA-SET-12 | PARTIAL | shots/EXTRA-SET-12-pass.png; `…-221729-EXTRA-SET-10.log` | In a small window (about 330×270 by eye, maybe smaller than the 500×400 target) with search active, content scrolls and nothing crashes. The no-search state and scroll-to-bottom per chip were not verified. |
| EXTRA-SET-13 | NOT RUN | — | Settings across windows. Budget. |
| EXTRA-SET-14 | NOT RUN | — | Reset All then restart. Reset All itself was used twice (T2-28, batch-4 pre-step) without a crash, but the restart check was not recorded. |

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
SI-1…5 were run in Batch 6b (Oct 8–9). The other §6 rows were not run as separate rows; the budget ran out after Batch 6 (see "Not tested"). Where an earlier row used the same steps, that is noted. It is not a §6 verdict.

| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| SIF-1 | NOT RUN (same steps as T1-15 → FAIL) | shots/T1-15-fail.png | T1-15 searched `中文` in the same `test_md/zz_cjk_search.md` (one line only; the Istanbul line was never added). No crash, but the caret landed at Ln 1 Col 1 (ISS-4). |
| SIF-2 | NOT RUN | — | |
| SIF-3 | NOT RUN | — | |
| SIF-4 | NOT RUN (identical to T1-16 → PASS) | shots/T1-16-pass.png | |
| SI-1 | **FAIL** — minor | shots/SI-1-fail.png; `…-235629-SI-1.log` | ISS-34. `time $FER /tmp/lf-test.md`: 0.095 s, rc 0, no 2nd process left. The primary logged "File already open, switching to window 0 tab 3", with no duplicate tab (tab part PASS). With the other Ferrite window (test_wrap_debug.md) on top, the lf-test.md window was **not raised**; the log shows only `request_user_attention` (Informational) ×2. Reproduced once more (0.080 s, same result). In T2-38 the window was raised, but it was the only Ferrite window then. |
| SI-2 | PASS | shots/SI-2-pass.png; `…-235629-SI-1.log` | Closed the tab, then `cd /tmp && time $FER ./lf-test.md`: 0.087 s, rc 0. The relative path resolved and the tab reopened with the fixture content; no "file not found". |
| SI-3 | PASS | shots/SI-3-pass.png, shots/SI-3-pass-closed.png; `…-235629-SI-1.log`, `…-235858-SI-3.log` | `--new-instance` gave a 2nd process and window. `instance.lock`/`.port`/`.pid` kept the primary's timestamps, and `instance.pid` = primary PID before and after the 2nd instance quit (exit 0; primary still running). Side findings for triage: the 2nd instance treated the primary's session.lock as stale, and on exit removed it and overwrote session.json/config.json (see observations). |
| SI-4 | PASS | shots/SI-4-setting-on.png, shots/SI-4-setting-persisted.png; `…-235629-SI-1.log`, logs/si4-secondary-ON-stderr.out | Settings › Files › "Allow multiple Ferrite windows/instances" is off by default. ON: a second launch spawned a 2nd process (no forwarding), and closing it left the primary running. OFF: forwarding came back (1 process, "File already open, switching…", 0.119 s). The toggle reached config.json only at a later save event (see observations). Settings was opened with the gear because Ctrl+, again did nothing. |
| SI-5 | PASS | shots/SI-5-pass.png; logs/ferrite-debug-20261009-000447-SI-5.log, `…-235629-SI-1.log` (killed primary, exit 137), logs/launcher-SI-5.out | `kill -9` on the primary left stale instance.* files. The next launch started over them with no hang and became the new primary (instance.pid and instance.port updated). It showed Session Recovery (answered Start Fresh). Forward check: 0.089 s, rc 0, 1 process. |
| PRI-1 | NOT RUN | — | |
| PRI-2 | NOT RUN | — | |
| PRI-3 | NOT RUN | — | |
| PRI-4 | NOT RUN | — | The Middle-click paste setting was toggled and persisted in EXTRA-SET-3, but its effect was not tested. |
| SPL-1 | NOT RUN | — | |
| SPL-2 | NOT RUN | — | |
| SPL-3 | NOT RUN | — | |
| SPL-4 | NOT RUN | — | |
| SPL-5 | NOT RUN | — | |
| SPL-6 | NOT RUN | — | |
| SPL-7 | NOT RUN | — | |
| SPL-8 | NOT RUN | — | |
| FNT-1 | NOT RUN | — | The Rendered view font was set to Inter and persisted (EXTRA-SET-2), but whether the Rendered pane changed font was not checked. |
| FNT-2 | NOT RUN | — | |
| FNT-3 | NOT RUN | — | |
| FNT-4 | NOT RUN | — | |
| IMG-X1 | NOT RUN | — | |
| IMG-X2 | NOT RUN | — | |

## Issues found
Severity follows the brief: Tier 1 FAILs are `blocker` (brief §8). Where the intrinsic severity (crash / major / minor) differs, it is given in brackets. Log excerpts ("last ~30 non-span lines" = `grep -v 'tracing::span' LOG | tail -n 30`, plus targeted excerpts around the failure) are in **Log excerpts per issue** below the table. Lines longer than 300 chars are truncated and marked.

| ID | Rows | Severity | Title | Exact repro steps | Expected | Actual | Evidence |
|----|------|----------|-------|-------------------|----------|--------|----------|
| ISS-1 | T1-2, T1-7, T1-17, T1-18, T1-18b, T1-22 (session 1); = EOL-2, RLD-8, ARR-6; also T2-35 / ARR-3 (ISS-31) | **blocker** (hang, cannot save/exit) | Rendered/Split edit session self-deadlocks the UI thread (`arrow_nav_sel_key` → parking_lot `lock_shared_slow`) | **A (ArrowDown):** 1. Open `test_md/test_find_arrow_nav.md`. 2. Switch to **Rendered**. 3. Click the first heading. 4. Press **ArrowDown** once (froze on the 1st press in the retry; T1-18: froze on the 2nd press toward the blockquote).<br>**B (commit by click-away):** 1. Create the CRLF fixture from the minimal test (`# CRLF Doc / First paragraph. / ## Heading Two / …`). 2. Rendered → click "First paragraph." → type ` edited`. 3. Click a blank area below the content. (Same with `t1-18b.md`: edit the callout body and click away.)<br>**C (dialog):** 1. Rendered, click a paragraph, type 2 chars, don't click away. 2. Ctrl+Shift+R → Unsaved Changes dialog → **Cancel**.<br>**D:** `test_mermaid_issue_165.md` Rendered → click a paragraph (enters edit mode) → click the Mermaid expand icon. | Navigation or commit works; the app stays responsive; edits can be saved. | The whole UI freezes: no input handled, log silent (crash-recovery snapshots stop), process alive in `futex_wait`, no panic. Requires `kill -9` (exit 137); unsaved Rendered edits survive only via crash recovery. Froze in 10 sessions (B ×4 incl. T1-18b, C ×2, A ×3, D ×1), 7 of them dumped with gdb. All **7 gdb dumps** show the main thread in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ← ferrite::markdown::editor::arrow_nav_sel_key ← remember_arrow_nav_sel ← {render_session_plain_text_block \| render_session_formatted_edit_text ← render_paragraph \| closure} ← MarkdownEditor::show_rendered_editor`. No other thread waits on a parking_lot lock. **Source pointer (from reading, not verified by a fix):** `src/markdown/editor.rs:710-716` calls `ui.memory_mut(\|m\| m.data.insert_temp(arrow_nav_sel_key(ui), block))`, and `arrow_nav_sel_key` (`:696`) calls `ui.memory(…)`. A read lock on the egui context memory is requested while the same thread holds the write lock, so it self-deadlocks. Computing the key before `memory_mut` should fix it. Nothing in the stack is Linux-specific, so **check Windows too**. **Batch 5 (T2-35, ISS-31):** 2 more freezes with Preview lock ON (arrow keys in Rendered), both dumped with the same stack, so there are 9 dumps in total. | shots/T1-2-fail.png, T1-2-blocked-hang*.png, T1-7-fail-hang*.png, T1-17-fail.png, T1-17-retry-fail-hang.png, T1-18-fail-hang.png, T1-18b-fail-hang.png, T1-22-fail-hang.png; logs/T1-2-hang-bt.txt, T1-7-hang-bt.txt, T1-17-hang-bt.txt, T1-17-retry-hang-bt.txt, T1-18-hang-bt.txt, T1-18b-hang-bt.txt, T1-22-hang-bt.txt; debug logs HANG-A, HANG-B, 204753-batch2, 210253-batch2, 210656-batch2c, 210920-batch2d, 213134-T1-22 (+ log excerpts below); T2-35 evidence under ISS-31 |
| ISS-2 | T2-36, VID-1, EXTRA-YT-2, EXTRA-YT-4, EXTRA-YT-6 (blocks VID-2/3/6-9/§9-§11, EXTRA-YT-1 A/C5-9/D, -3, -5, -8, EOL-7 resize) | **blocker** (crash) | **Linux:** any trusted YouTube embed aborts the app: `GDK has not been initialized. Call gdk::init or gtk::init first.` | Linux X11 build (Windows uses WebView2 and is probably not affected): 1. Create a `.md` containing only `{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ}}` (or open `test_md/test_video_embeds.md`). 2. Switch to **Rendered** or **Split**. → abort. Also: in Split, type a trusted `{{video https://youtu.be/… width=320}}` line; the app aborts on the keystroke that completes it. Offline makes no difference. | Thumbnail plus play overlay; WebView on play, or degrade to a thumbnail if the WebView can't be created; never a crash. | Panic at `gdk-0.18.2/src/auto/display.rs:301:9` on the **first render** of the embed (before any click), exit 134 SIGABRT. Backtrace: `gdk::auto::display::Display::default ← ferrite::markdown::video_render::create_child_webview ← ferrite::markdown::editor::render_video_embed ← render_node ← … ← MarkdownEditor::show_rendered_editor`. `rg 'gtk::init\|gdk::init\|gtk_init' src/` finds nothing. wry's `build_as_child` needs GTK initialised on Linux. Untrusted hosts (vimeo, nocookie, evil-suffix, no-id, playlist-only) and invalid schemes correctly never create a WebView (VID-4, EXTRA-YT-1 B/C: PASS). Reproduced in 7 launches. | shots/T2-36-fail.png, VID-1-fail-crash.png, EXTRA-YT-2-fail-offline-crash.png, EXTRA-YT-4-fail-crash.png, EXTRA-YT-6-fail-crash.png; debug logs 223058-T2-36, 223148-T2-36-retry, 223237-T2-36-split, 223520-VID-1, 223923-EXTRA-YT-4, 223957-EXTRA-YT-6, 224251-EXTRA-YT-2 |
| ISS-3 | T1-4b (must-pass) | **blocker** [major: file can't be opened] | UTF-16 LE file (with BOM) is classified as binary and handed to the OS default app | 1. Create a UTF-16 LE + BOM file with CRLF text (Notepad → Save As → UTF-16 LE; Linux: `printf '\xff\xfe' > f.md; iconv -f UTF-8 -t UTF-16LE crlf-test.md >> f.md`). 2. Open it in Ferrite (CLI argument or forwarded open). | Opens as text in a tab. Raw → Enter → Ctrl+S keeps `0D 00 0A 00` pairs. | Log: `Delegating to external app (binary): /tmp/crlf-utf16.md`. LibreOffice Writer opened it and no Ferrite tab appeared. `src/state.rs:278 is_binary_content()` returns true for **any `0x00` byte** (`bytes.contains(&0)`, line 285) before any BOM or encoding detection. Every UTF-16 file contains 0x00 bytes. | shots/T1-4b-fail.png; logs/T1-4b-check.txt (fixture valid); debug log 202232-T1-4b |
| ISS-4 | T1-15 (must-pass); SIF-1 | **blocker** [major] | Search in Files: clicking a CJK match opens the file with the caret at Ln 1, Col 1 instead of on the match | 1. Open the repo folder as workspace. 2. Create `test_md/zz_cjk_search.md` with `搜索测试 中文 search`. 3. Ctrl+Shift+F → `中文` → click the result for that file. | Caret (and highlight) **on** `中文` (Ln 1, Col 6). | No crash. The file opens, but the caret is at Ln 1, Col 1. The log shows the right target (`Opened file from search: … at line 1, char offset 5`, `Set transient highlight at 5..7`), and then the editor resets the cursor: `scroll_to_line 1 … cursor_before=Cursor{line:0,column:0}`, `after set_cursor, cursor_after=Cursor{line:0,column:0}`, `cursor sync changed: (0, 5) -> (0, 0)`. Reproduced twice. | shots/T1-15-fail.png; debug log 204753-batch2 (lines quoted below) |
| ISS-5 | T1-14; = IMG-3, IMG-9 | **blocker** [major on Linux; Windows unknown] | Ctrl+V with an image on the clipboard inserts nothing on X11; the context-menu Paste works | 1. Open a saved `.md`, caret at the end of `日本語テキスト ééé`. 2. Put a PNG on the clipboard (Snipping Tool; Linux: `xclip -selection clipboard -t image/png -i test_md/assets/sample.png`). 3. Press **Ctrl+V** (twice). 4. Then right-click › Paste. | Ctrl+V inserts `![](assets/…png)` at the caret, writes the PNG under `assets/`, and it renders. | Ctrl+V: nothing inserted, no PNG; log `ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` (×2). Right-click › Paste: inserted `![](assets/1791485625-clipboard.png)` and wrote `fixtures/assets/1791485625-clipboard.png` (6190 bytes). | shots/T1-14-fail.png; debug log 204753-batch2 |
| ISS-6 | T1-19; = UND-1 | **blocker** [major] | Split view: a preview code-block edit is undone one keystroke at a time instead of as one edit (#167) | 1. Open `test_md/test_copy_and_split_undo.md`, Split view. 2. In the preview, edit the Python code block in Part B: add a line `    # appended`, click Done. 3. Press Ctrl+Z once. | The whole edit reverts in one undo step. | Only the last char is removed (`# appende`), status "Undo (14 remaining)". The log shows 15 separate `Content modified in split rendered pane, recorded for undo` entries (19:16:08–09Z) for one edit. The first attempt, typing a newline in the preview, lost text and left blank lines. SENTINEL lines stayed intact. | No screenshot saved; debug log 211300-batch2e |
| ISS-7 | T1-21 (must-pass); = MMD-12 | **blocker** [major; no crash] | Typing a backtick fence character by character: auto-close inserts a backtick after every keystroke and the text comes out reversed | 1. Ctrl+N, Split view. 2. Type character by character: ```` ```mermaid ````. 3. Continue with `flowchart TD`, etc. | Fence typed as written; banner may flicker; diagram renders when the fence closes; never crashes. | No crash (exit 0; malformed `A}x{ --> B` and `subgraph a] [b` don't crash either). But the opener became about 14 backticks followed by `diamrem` (mermaid reversed), and later lines landed at stale positions with more stray backticks. The log has **66** `` Auto-close: Inserted '`' after '`' `` lines, one after every keystroke: content grows by 2 while the caret advances by 1. Typing `flowchart TD` inside a pasted opener gave 11 auto-close inserts for 12 chars. Reproduced 3 times. After an Enter before the closing fence, the preview drew a stray ```` ``` ```` node. A pasted clean block renders correctly. **Caveat:** input was injected synthetically, so please confirm with a real keyboard; the log shows Ferrite's own auto-close doing the inserts. | shots/T1-21-fail.png, T1-21-malformed.png, T1-21-supp-clean-paste-diagram.png; debug log 212111-T1-20 |
| ISS-8 | T1-22; = MMD-8 (MMD-1 likely) | **blocker** [major; hang part = ISS-1] | Mermaid #165: popup never opens; no Fit/Native toggle; no hover tooltip or hand cursor | 1. Open `test_md/test_mermaid_issue_165.md`, Rendered. 2. Hover the widest flowchart (FC-165a). 3. Click the expand icon, the diagram body, and Ctrl+click it. | Hover shows the Fit/Native toggle plus the tooltip "Click diagram or Ctrl+Click to open popup" and a hand cursor; a click opens the zoom/pan popup. | Fit-to-pane OK, no gap below, no overpaint, wheel-scroll OK. But: no tooltip or hand cursor on hover; the header has only "Flowchart \| Source > mermaid" and the expand icon, and the source has no Fit/Native label; **6 clicks plus 1 Ctrl+click never opened the popup** (zoom/close untestable). Session 1 hung (ISS-1) when the expand icon was clicked while a paragraph was in edit mode. Session 2 (about 6 min of clicking without editing) did not freeze. | shots/T1-22-fail.png, T1-22-fail-hang.png, T1-22-probe.png, T1-22-hover-probe.png; logs/T1-22-hang-bt.txt; debug logs 213134-T1-22, 213643-T1-22b |
| ISS-9 | EXTRA-SET-9 | **major** (settings data loss) | A corrupt or wrong-typed `config.json` silently resets **all** settings to defaults and overwrites the file on exit | 1. Quit Ferrite. 2. Edit `config.json`: either replace it with `{ not json`, or set `"font_size": "abc"` in an otherwise good file. 3. Launch Ferrite, then quit cleanly. 4. Inspect `config.json`. | Bad field ignored or defaulted individually, or a `.bak` restore; the user is told (toast); the original file is not destroyed. | Only a log WARN (`contains invalid JSON: key must be a string at line 1 column 3` / `invalid type: string "abc", expected f32 … Using default.`). No toast, no `.bak` (none exists). **One bad field discards the whole config.** On exit `Configuration saved successfully`: config.json is rewritten with defaults, and the user's settings are gone for good. No crash or hang. | shots/EXTRA-SET-9a-pass.png; debug logs 221551-EXTRA-SET-9a, 221645-EXTRA-SET-9b |
| ISS-10 | EXTRA-SET-2-defect | **major** | Raw editor text invisible with Dark theme + Syntax Theme "ANSI" | 1. Settings: Theme Dark, Font JetBrains Mono, Font Size 20, Syntax Theme **ANSI**. 2. Open any `.md` in Raw. | Readable text. | Body text invisible; line numbers and minimap still drawn. Reproduced 2×. Switching to Base16 Mocha Dark makes the text visible again immediately. Light + ANSI not tested. | shots/EXTRA-SET-2-fail-ansi-invisible-text.png vs shots/EXTRA-SET-2-pass-mocha-visible.png; debug log 220315-EXTRA-SET-2-relaunch |
| ISS-11 | EXTRA-SET-4 (restart) | minor | "Recent Files: Remember last 0 files" is not honoured after restart | 1. Settings › Files › Recent Files slider → 0 ("Remember last 0 files"). 2. Quit cleanly, relaunch, open Settings › Files. | Slider still at 0. | config.json before relaunch had `max_recent_files=0`; after relaunch the slider reads "Remember last **10** files" (recent list empty, no crash). The other 4 Files settings persisted. | shots/set-persist-recent.png; debug log 222853-set-persist |
| ISS-12 | EXTRA-SET-1 (+ EXTRA-SET-3 note) | minor | Esc does not close the Settings tab; Ctrl+, did not open Settings | 1. Open Settings (ribbon gear). With no text field focused, press **Esc**. 2. With a document active, press **Ctrl+,**. | Esc closes Settings; Ctrl+, opens or focuses it. | Esc: no effect (1 attempt). Ctrl+,: no effect (several attempts across rows, and again in SI-4). **Possibly injection-related** (comma keysym with Ctrl through synthetic input), so re-check on a real keyboard. Also: clicking the gear while Settings is active closes it (toggle). | debug log 221729-EXTRA-SET-10 (no WARN for these actions) |
| ISS-13 | T2-29 | minor (needs triage: may be a stale checklist expectation) | No terminal-shell selector in Settings | Settings → Terminal chip, scroll to the bottom; search `shell`. | Row 29 lists "terminal shell" among the 0.3.0 controls. | Terminal shows Enable … Focus Terminal on Prompt, with no shell selector. Search `shell` → only "Code execution" and "Enable Integrated Terminal". The settings registry has no shell entry either. | shots/T2-29-pass-shell-search.png, T2-29-terminal-*.png; debug log 215656-set-extra |
| ISS-14 | T2-25b | minor | Settings "All" overview: wheel-scroll stops at Files › Auto-save delay; Recent Files, Keyboard and Terminal can't be reached in the All view | Settings, empty search, All → wheel-scroll to the bottom. | Every section reachable. | Scroll ends at "Auto-save delay". The missing sections are reachable through their chips. Needs a second look on a real display. | shots/T2-25b-pass.png; debug log 214507-T2-25-relaunch |
| ISS-15 | T2-23, T2-24, EXTRA-SET-10 | minor | Keyboard chip live count is wrong | Settings → query `zoom` (or `tab`). | Chip count = number of matching rows. | Keyboard chip shows (1) while 3 (`zoom`) or 4–5 (`tab`) rows show. | shots/T2-23-pass-zoom.png, T2-24-pass.png |
| ISS-16 | T2-25 | minor (doc/code mismatch) | "Recently changed": shortcut rebinds not listed; checklist cap 6 vs code cap 8 | Change theme, font size and a shortcut → reopen Settings. | Row 25: all three listed, max 6. | Only Theme and Font Size listed; the rebind (Toggle Word Wrap → Ctrl+Alt+K) is not (code excludes rebinds by design). Code: `MAX_RECENTLY_CHANGED = 8` (`src/ui/settings/registry.rs:18`); the GUI showed at most 2 entries, so the cap wasn't exercised. *Accent:* after EXTRA-SET-2 the block **did** list "Ferrite accent", so accent is not missing in this build. | shots/T2-25-pass.png |
| ISS-17 | T2-28 | minor | Settings › Reset All has no confirmation | Settings → Reset All. | Confirm dialog (checklist: "Reset All → confirm"). | Resets immediately (status "Settings reset to defaults"). Side effect seen later: it re-enabled "Restore previous session". | shots/T2-28-pass.png |
| ISS-18 | T2-23, T2-27, EXTRA-SET-10 | minor (possibly input-injection) | First keystroke(s) dropped in Settings search and in the shortcut-capture popup | Type `bold` into Settings search right after Ctrl+A; open a chord-capture popup and press a chord once. | Every key lands. | `bold` → `ld`; the first chord press was swallowed (the second worked); `[` dropped as the first char. Synthetic input may contribute; re-check on a real keyboard. | debug logs 213643-T1-22b, 214507-T2-25-relaunch, 221729-EXTRA-SET-10 |
| ISS-19 | EXTRA-SET-8 | minor | No validation or feedback for Terminal startup command, Custom Prompt Patterns (invalid regex), Sound file path | Enter `([` in Custom Prompt Patterns; `/nonexistent.wav` as sound file; `;;;` as startup command. | Invalid values rejected or flagged. | Accepted silently, no indicator (no WARN in the log). Max Line Width, by contrast, clamps correctly (0 → 400, -5 → 400, 99999 → 2000, `abc` ignored). | debug log 220823-EXTRA-SET-3-relaunch |
| ISS-20 | T1-11 | polish | Session Recovery dialog shows the raw i18n key `recovery.session.tabs_unsaved` | Edit a saved tab, kill the process, relaunch. | Translated sentence. | Literal `recovery.session.tabs_unsaved` between the dialog sentences. | shots/T1-11-pass.png |
| ISS-21 | T1-8 → T1-10 (observation) | minor | A tab closed during the session reappears in the next session-restore/recovery tab list | Open A and B; close B; kill and relaunch. | B not restored. | `other-tab.md` (closed in T1-8) is listed in "Restoring N tab(s)" and in the tab bar of the recovery state. It disappears after Restore Session. Disk untouched. | debug logs 203037-T1-10, 203215-T1-11 |
| ISS-22 | T1-11 | minor | "Recovered content differs from this file on disk" banner stays after Ctrl+Z makes the buffer equal to disk | Kill → Restore Session → Ctrl+Z once. | Banner disappears. | Banner remains. | shots/T1-11-pass.png; debug log 203534-T1-11-dialog |
| ISS-23 | T1-11 | minor | Stale recovery file left after a clean exit | After the T1-11 restore + undo, Alt+F4 (no dialog, exit 0). | `recovery/` cleaned. | `~/.config/ferrite/recovery/12.json` left behind containing the undone edit `KILL-EDIT` (T1-10 left the dir empty). | debug log 203534-T1-11-dialog |
| ISS-24 | T1-16, EXTRA-SET-3, EXTRA-SET-8, batch 4 | minor (may be partly environmental) | Repaint lag: UI updates only after the pointer moves | Find results, checkbox toggles, typed text in Settings fields, Rendered/Split pane. | Immediate repaint. | Stale frame until a mouse move ("No matches" shown until the pointer moved; checkbox looked unchanged). Seen only on Xvfb + llvmpipe. | shots/T1-16-pass.png |
| ISS-25 | T1-13, EXTRA-SET-3 | polish | Stale status-bar info | Duplicate a line (Ctrl+Shift+D); turn Vim mode off with the Settings tab active. | Status bar updates. | Line count stayed 10 while 11 lines showed; the Settings-tab status bar kept `[NORMAL]` with Vim off. | debug log 210253-batch2 |
| ISS-26 | VID-5 | minor | Bare URLs and `<https://…>` autolinks are not link-styled in Rendered | Rendered doc with a bare `https://…` and `<https://…>`. | Rendered as links. | Plain black text (`[text](url)` is blue). A control doc without video behaves the same. | shots/VID-4-pass.png; debug log 223549-CTRL-plain |
| ISS-27 | T2-29 | minor | Open Workspace and Close Workspace are both bound to F12 | Settings › Keyboard. | Unique default bindings. | Both rows show F12. | shots/T2-29-*.png |
| ISS-28 | Automated gate | minor (test hygiene) | `cargo test` writes to the real user config dir | Run `cargo test` on a machine with no `~/.config/ferrite`. | Tests use a temp dir. | `~/.config/ferrite/config.json` and `recovery/` created (prep observation, gui-test-plan §1; the dir existed before the first GUI launch and was moved aside). | logs/cargo-test.log, results-batch1.md Batch 0 |
| ISS-29 | EXTRA-SET-10 | polish | Emoji 🙂 accepted but invisible in the Settings search field | Paste 🙂 into Settings search. | Glyph visible (Noto Color Emoji installed). | Blank field; every launch logs WARN `epaint … Failed to find replacement characters`. | shots/EXTRA-SET-10-pass.png |
| ISS-30 | T2-32; = EMO-1, EMO-2 (related: ISS-29) | major | Emoji are drawn monochrome and some common emoji are not drawn at all, although Noto Color Emoji is installed and registered | 1. Linux: check that `fc-list \| grep -i emoji` lists `/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf` (Windows: Segoe UI Emoji is present by default). 2. Open `test_md/test_emoji.md` in **Raw**. 3. Look at line 9 (`## Common single-codepoint emoji`: 😀 😅 🤔 😴 🥳 😢 😡) and at the symbols, skin-tone, ZWJ and flag lines. 4. Switch to **Rendered** and compare. | Every emoji visible, in colour since a colour emoji font is installed; no blanks or tofu; UI icons unchanged. | Raw: only 5 of the 7 common emoji are drawn, and the last two positions are blank (no glyph, no tofu box). All glyphs are monochrome. Rendered: the same line has 5 glyphs, with blanks at positions 4 and 7 (identical after about 1 min). The variation selector after ⚠ shows as an empty box; ZWJ sequences split into component glyphs with one empty box; flags show as letters `N O U S J P D E`. Toolbar and tab icons are fine. Log: "Found system emoji font at … NotoColorEmoji.ttf", "Registered system emoji font fallback", "Pre-warmed font atlas with … 9 emoji characters"; every launch also logs WARN epaint "Failed to find replacement characters". No crash, exit 0. | shots/T2-32-raw.png, T2-32-rendered.png, T2-32-rendered-recheck.png; logs/T2-32-fail-tail.txt; debug log 230020-T2-32 |
| ISS-31 | T2-35; = ARR-3 (same root cause as ISS-1) | **blocker** (hang; known ISS-1 deadlock) | Preview lock ON: arrow keys don't navigate the Rendered view, and after a click into the pane one ArrowDown freezes the app | 1. Open `test_md/test_find_arrow_nav.md`. 2. Turn **Preview lock** ON (padlock). 3. Switch to **Rendered**. 4. Press ArrowDown 3 times. 5. Click a blank area of the Rendered pane (not on text) and press ArrowDown once. | Arrows move/scroll the Rendered view; edits stay blocked; the app stays responsive. | Step 4: no visible scroll (before/after screenshots pixel-identical). Step 5: the UI froze, the log stopped advancing, the process stayed alive; kill -9 (exit 137). gdb: main thread in `parking_lot::raw_rwlock::RawRwLock::lock_shared_slow ← ferrite::markdown::editor::arrow_nav_sel_key ← remember_arrow_nav_sel ← … ← MarkdownEditor::show_rendered_editor`, the same stack as ISS-1. Froze in 2 launches (the first on arrow keys in Rendered, the second after the click + 1 Down). The lock itself works: Rendered typing is blocked, the Raw/Split-left pane stays editable, and the lock persists across restart. The ISS-1 fix should cover this. | shots/T2-35-fail.png, T2-35-rendered-lock-on.png, T2-35-rendered-lock-typed-Q.png, T2-35-split-raw-typed-Z.png; logs/T2-35-hang-bt.txt, logs/T2-35-relaunch-hang-bt.txt; debug logs 230821-T2-35, 231259-T2-35-relaunch-a |
| ISS-32 | T3-43; = MMD-7 | major | Mermaid flowcharts with subgraphs: edges cross subgraph titles, nested subgraphs overlap or are laid out flat, and one diagram drops a node, an edge and a label | 1. Open `test_md/test_flowcharts.md`, **Rendered**. 2. Look at "Subgraph Test (TD)", "Edge Routing Across Subgraph Boundaries (TD)", "Nested with Direction Override" and "Asymmetric Shape with Dash-Style Edge Labels". 3. Then "Nested Subgraph Test (Sibling Subgraphs)", "True Nested Subgraphs" and "Deeply Nested Subgraphs (3 Levels)". | Edges avoid nodes and titles (FC-83a regression); each layout matches the block's Expected text. | (1) Subgraph Test (TD): the Node 2 → Node 3 arrow runs through the "Inner Group" title. (2) Edge Routing (TD): Start → Step 1 crosses the "Process A" title, and Step 2 → Step 3 crosses "Process B". (3) Nested with Direction Override: the Input Stage box overlaps the Main Process container and hides its title; Output Stage sits outside Main Process. (4) Asymmetric Shape with Dash-Style Edge Labels: the "Two line / edge comment" label is clipped behind Result One and hides the Odd → Result One arrow; "Simple label" on Odd → Result Two is not drawn; node "Flag" (A) and its dotted edge to Target are not drawn. Also: Sibling Subgraphs draws `one --> two` as two extra boxes "one"/"two" instead of linking the subgraphs and ignores `direction LR`; True Nested and Deeply Nested show the subgraphs side by side, not nested. No error boxes, no crash. Many other diagrams are clean (list in `results-batch6.md`). | shots/T3-43-fail-subgraph-test-td-title-overlap.png, T3-43-fail-edge-routing-td-title-overlap.png, T3-43-fail-nested-direction-override-overlap.png, T3-43-fail-dash-edge-label-overlap.png, T3-43-flowchart-subgraph-td.png, T3-43-flowchart-complex-cross.png; debug log 234101-T3-43 |
| ISS-33 | T3-44; = HTML-1 | major | Two adjacent single-line `<div align>` blocks are not rendered; the document's first line is drawn in their place | 1. Open `test_md/test_github_html.md`, **Rendered**. 2. Scroll to "Adjacent divs", which contains `<div align="center">First centered block.</div>`, a blank line, and `<div align="right">Second block, right-aligned.</div>`. | "First centered block." centred and "Second block, right-aligned." flush right. The repo docs (`docs/technical/markdown/github-html-block-subset.md`, test `test_html_single_line_div_align`) say this single-line form is supported. | Neither text appears. The document's first line, `A# GitHub HTML Subset Test` (that is the literal first line of the committed fixture), is drawn twice in their place, once indented. Multi-line `<div align>`, `<details>` and `<kbd>` render correctly. No crash. Side note: `<strong>` inside a `<summary>` shows as small grey tag chips, and the summary text is not bold. | shots/T3-44-fail-adjacent-single-line-divs.png, T3-44-adjacent-divs-check.png; debug log 234543-T3-44 |
| ISS-34 | SI-1; = MW-6 (partly) | minor (possibly WM-related) | A forwarded open selects the right tab but doesn't raise its window when another Ferrite window is on top | Linux X11 (xfwm4): 1. `ferrite /tmp/lf-test.md`. 2. Have a second Ferrite window (e.g. Ctrl+Shift+N, or a restored 2-window session) and bring it to the front. 3. From a terminal: `time ferrite /tmp/lf-test.md`. Windows: same with two Ferrite windows and Explorer → Open. | The window holding lf-test.md is raised and focused (#147); the 2nd process exits in ≲1 s. | The 2nd process exits in 0.095 s (rc 0), and the primary logs "File already open, switching to window 0 tab 3", but the active X window stays the other Ferrite window. The log shows only `winit::Window::request_user_attention; request_type=Some(Informational)` ×2, with no focus or raise request. Reproduced (0.080 s). In T2-38, with a single Ferrite window, the window **was** raised. This may be xfwm4 focus-stealing prevention turning the raise into an attention hint, so check on Windows and another WM before fixing. | shots/SI-1-fail.png; debug log 235629-SI-1 |
| ISS-35 | T3-41 (PARTIAL, supplementary fixture); TBL-4 | minor | Bold inside a link in a table cell shows literal `**` | 1. Open `test_md/task_50_table_inline_formatting.md`, **Rendered**. 2. Find the cell `[**bold link**](https://example.com)`. | Bold, underlined link text "bold link". | Underlined link text with the `**` asterisks visible. Other inline formatting in cells (bold, italic, strikethrough, bold+italic, code, strike+code, plain links) renders styled. Found on a supplementary fixture, so it is not a row FAIL. | shots/T3-41-supp-inline-format-task50.png; debug log 233908-T3-41 |

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


The other issues (ISS-13 to ISS-29, and ISS-35) are minor or polish. Their evidence is the screenshot or log named in the table. The full debug logs are in `logs/`.


## Not tested and why
The first pass ended after Batch 4 (YouTube). A follow-up on Oct 8–9 ran Batches 5–6: Tier 2 rows 30–35 and 37–40, Tier 3 rows 41–47, and §6 SI-1…5. The budget ran out after Batch 6. Following brief §8, the report was written with partial coverage instead of continuing. Not reached:

- **§6 extra rows, 22 of 27** (budget):
  - Batch 7: SIF-1…4, PRI-1…4, IMG-X1…2. SIF-1 and SIF-4 repeat T1-15 (FAIL) and T1-16 (PASS). PRI rows need X11 primary selection, which this box has, so they are testable next time.
  - Batch 8: SPL-1…8, FNT-1…4.
  - SI-1…5 were run (Batch 6b).
- **Minimal row 40**: SKIPPED (platform), because it is the Windows title-bar row. Its Linux counterpart, TTL-6, was not reached (budget).
- **Total test, Batches 9–12 remainder** (rows not covered by a minimal or §6 row; budget):
  - §1: STAT-3, EOL-3, EOL-5, EOL-9, SES-1 (T1-9 covered two saved tabs, not the untitled/Save As case), SES-3, SES-4, SES-5, SES-6. STAT-4 optional.
  - §2: ENT-6, FIND-5, ARR-1, ARR-2, ARR-4, ARR-5, ARR-6 (second half, `## #1 issue`), CPY-2, CPY-3, UND-3. ARR-1/ARR-2 would probably hit ISS-1, since T1-17 and T2-35 froze on ArrowDown in Rendered. (ENT-1…5, FIND-1…4, ARR-3 and CPY-1 are covered by rows 31, 30, 35 and 34.)
  - §3: RLD-3, RLD-4, RLD-5, RLD-7 (UTF-16 / CP-1252 reload; UTF-16 is blocked by ISS-3), IND-1…5, IND-7…9, EMO-4, EMO-5, IMG-2, IMG-4, IMG-5, IMG-7, IMG-8 (IMG-8 maps to IMG-X1, also not reached). (EMO-1…3 and IMG-1/6 are covered by rows 32 and 33.)
  - §4: MMD-1…4, MMD-6, MMD-9…11, MMD-13. MMD-5/7 are covered by row 43, MMD-8 and MMD-12 by rows 22 and 21.
  - §5: VID-10 is not recorded separately. All other VID rows are above.
  - §6: MW-5. MW-1…4 are covered by row 37, MW-6 by row 38 and SI-1.
  - §7: HTML-4…6. HTML-1…3 are covered by row 44; HTML-6 (single-line `<div align>` coalescing) is likely affected by ISS-33.
  - §8: TBL-2, TBL-3, CSV-4, CSV-5. TBL-1/4 are covered by row 41, CSV-1…3 by row 42.
  - §9: UX-4, UX-7, UX-8, UX-9, UX-11. UX-1/2/3/5/6/10 are covered by rows 35, 39, 39, 46, 27 and 47.
  - §10: WS-1, WS-2, RUN-1, RUN-4, RUN-5, GIT-2, GIT-3. RUN-3 is covered by row 45.
  - §11: TTL-6 (Linux, budget).
  - §12: PLT-2 is not recorded separately (see the total-test table).
- **EXTRA rows not reached:**
  - EXTRA-SET-5 (Terminal persistence), EXTRA-SET-6 (Keyboard persistence), EXTRA-SET-7 (language switch), EXTRA-SET-13 (Settings across windows), EXTRA-SET-14 (Reset All then restart): budget.
  - EXTRA-YT-7: budget, and it needs players.
- **Partial or blocked parts of rows run in Batches 5–6:**
  - Row 34 BLOCKED: six synthetic mouse drags never selected the whole bash block in Rendered. The block's Copy button gave byte-identical output, but that is not the row's select-and-copy path.
  - Row 39 PARTIAL: Alt+Z is unverified. The synthetic Alt+Z typed a plain `z`. ToggleWordWrap is bound to Alt+Z in `src/config/settings.rs:906`, so it needs a real-keyboard check. Everything else in the row passed.
  - Row 41 PARTIAL: `test_tables.md` has no `:-:`/`--:` column and no formatted cells. The supplementary fixtures render alignment correctly, but `[**bold link**](url)` shows literal asterisks (ISS-35).
  - Row 47 PARTIAL: the pdf and png viewer tabs work. `.zip` and `.exe` were handed to xdg-open; on this box that is Chrome, which opened and downloaded them. The Ferrite toast was not observed because Chrome covered the window.
  - Row 46: the Don't Save step couldn't be done, because closing the modified Untitled doc gave no prompt (see observations).
- **Blocked by ISS-2 (GDK crash):**
  - VID-2, VID-3, VID-6, VID-7, VID-8, VID-9, VID-§9, VID-§10, VID-§11.
  - EXTRA-YT-1 cases A2–A14, C5–C9 and D1–D8; EXTRA-YT-3, EXTRA-YT-5, EXTRA-YT-8.
  - The drag-resize half of EOL-7.
  - The play, overlay, resize and source-update steps of row 36.
  - The play-video sub-step of row 37.
- **Blocked by ISS-1 (deadlock):**
  - Row 2: Ctrl+S and EOL check never reached.
  - Row 17: paragraph-integrity check.
  - Row 18: blockquote merge check.
  - Row 18b: container-prefix check for heading and quote.
  - Row 22: popup zoom/close, and the popup never opened anyway.
  - Row 35: arrow-key navigation with Preview lock ON (froze twice, ISS-31).
- **Blocked by environment:** row 20 (Alt+click captured by xfwm4 `easy_click`); command palette paths in EXTRA-SET-1 (Alt+Space captured by xfwm4); row 34 drag-select and row 39 Alt+Z (synthetic input); row 47 toast (Chrome is the default handler and covers the window).
- **Skipped by platform rule (brief §5):** row 40, TTL-1…TTL-5, MAC-1…MAC-4, PLT-1, PLT-3, RUN-2 `(Windows)`, GIT-1 (Windows `\\?\` paths).
- **Artifacts:** the earlier single `ferrite-qa-v0.3.1-linux-artifacts.zip` was too big to attach and has been replaced by four attachments, each under 24,000,000 bytes (ZIP deflate, level 9): `qa-shots-1.zip`, `qa-shots-2.zip`, `qa-shots-3.zip` (all 183 files of `shots/`, split alphabetically: `EXTRA-SET-10-pass.png` … `T2-23-pass-zoom.png`, `T2-23-pass.png` … `T2-35-clean-a-start.png`, `T2-35-clean-relaunch-rendered.png` … `set-persist-recent.png`) and `qa-logs.zip` (all 115 files of `logs/`, plus `results-batch1.md`…`results-batch6.md`, `gui-test-plan.md`, `report-format.md`). Nothing was trimmed. `launcher-*.out`, `T2-25-relaunch.out` and `T3-4x-run*.out` are the launcher's stdout copies of the matching debug logs.

## Tag-gate opinion

No. Tier 1 is not green on Linux, and I would not tag v0.3.1 from commit `8e6ef508`. Against the minimal test's "Tag gate (minimal)", item 1 fails outright: of 25 Tier 1 rows, 13 pass, 11 fail and 1 is blocked by the test environment. Three of the six rows the brief says must now pass (4b, 15, 21) fail, and a fourth (38) was not reached. Item 2, Settings rows 23–29, is close: 23–28 pass with minor deviations, and 29 is partial because there is no terminal-shell selector, which may be a stale expectation. Item 3 can't be judged because most of Tier 2 and Tier 3 was not reached. The decision comes down to two blockers, not the pile of smaller findings. (1) The Rendered/Split edit session deadlocks the UI thread. One ArrowDown in Rendered, clicking away after a Rendered edit, or Cancel in the reload dialog freezes the app until it is killed. All seven backtraces stop in `arrow_nav_sel_key` ← `remember_arrow_nav_sel`, where the source calls `ui.memory()` inside a `ui.memory_mut()` closure. Nothing in that path is Linux-specific, so it needs to be confirmed on Windows before relying on the earlier Windows pass. (2) On Linux, rendering any trusted YouTube embed aborts the process, because GTK/GDK is never initialised before wry creates the child WebView. Before tagging I would fix, in this order: the deadlock (compute the memory key before taking the write lock); GTK initialisation on Linux, with a thumbnail-only fallback and never a panic when the WebView can't be created; BOM/UTF-16 detection before the binary check (row 4b); the Search-in-Files caret reset (row 15); and backtick auto-close while typing a fence (row 21, confirmed with a real keyboard). After that come the remaining Tier 1 failures (Ctrl+V image paste on X11, split-view undo coalescing, the Mermaid popup and Fit/Native toggle) and the silent config wipe. Then re-run all of Tier 1 on both Linux and Windows. On Linux, use real keyboard input and free the Alt+click grab so row 20 can run. Finish with rows 23–40, the video rows, and the §6 extras that were never reached. Update after the Tier 2/3 follow-up (Oct 8–9): Tier 2/3 found no new crash, its only hang (T2-35) is the same `arrow_nav_sel_key` deadlock, the other new FAILs are mainly rendering-fidelity issues (emoji glyphs, Mermaid subgraph layout, single-line `<div align>`) plus a window-raise gap in single-instance forwarding, and must-pass row 38 now passes. So the tag decision still hinges on the Tier 1 blockers: the deadlock and the YouTube GDK crash.
