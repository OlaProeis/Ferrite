# Ferrite v0.3.1 QA re-run report (round 2 close-out) — Linux

- **Commit tested:** code `221b6ae` (round-2 close-out), branch `0.3.1-experimental`, HEAD `9effd76` (`docs(qa): point GrokBot re-run gate at 221b6ae+`; HEAD only changes the re-run doc). Working tree clean at start and at the end of every batch (`git status --short` empty). Binary `target/release/ferrite`, `ferrite 0.3.1`.
- **Gate doc followed:** `grokbot/New/rerun-v0.3.1-round2.md` (pass criteria, tables, suggested run order). Baseline: `qa-report-v0.3.1-linux.md` (commit `8e6ef508`, Oct 8–9 2026).
- **Date / timezone:** Monday 2026-11-09, GUI testing 10:39–13:51 CET (Europe/Oslo, UTC+1). Times in this report are CET unless they end in `Z`; debug-log timestamps are UTC (CET = Z + 1 h).
- **Environment:** Debian GNU/Linux 13 (trixie), kernel `6.12.113+`. **X11 only:** Xvfb display `:2`, 1920×1200×24, xfwm4, no compositor. **No GPU:** Mesa **llvmpipe** software GL (eframe glow). WebView stack WebKitGTK 4.1 via wry (see version note in §7).
- **Toolchain:** `rustc 1.92.0 (ded5c06cf 2025-12-08)`, `cargo 1.92.0`.
- **Build time (release):** 3 m 38 s (`cargo build --release --locked`, first build, binary mtime 10:32 CET); rebuild after the box restart 3 m 33 s (binary mtime 11:48 CET, same size 46 626 640 bytes).
- **Tester:** Grok Bot, driving the GUI with synthetic X11 input (xdotool / computer-use) through `run-ferrite.sh` (`--log-level debug`, exit code recorded). Tester only: no Ferrite code was changed, nothing was committed, pushed or filed.
- **Raw results:** `rerun2/results-A-iss1.md` … `results-H-spot-si-ses6.md`, `automated-gate.md`, `env-restore.md` (all also inside `rerun2-logs.zip`).
- **Attachments:** `rerun2/rerun2-shots.zip` (screenshots) and `rerun2/rerun2-logs.zip` (debug logs, check files, gate logs, raw batch results). Paths in the Evidence columns below are the paths **inside the zips** (`shots/…`, `rerun2/…`, `logs/…`).

## 0. Verdict

**NOT ready to tag.** The pass criteria in `rerun-v0.3.1-round2.md` are not met: **23 rows FAIL** (each filed below as a new issue, ISS-64 … ISS-75; the round-2 PRD was not reopened) and Tier 1 has **5 FAILs**.

**Blocking Tier 1 FAILs:**

| Tier 1 row | New issue | Summary |
|---|---|---|
| T1-14 | ISS-66 | Ctrl+V of a PNG inserts nothing on X11 (arboard: "incorrect type received from clipboard"); no toast either |
| T1-17 | ISS-64 | Rendered ArrowDown from the first heading never reaches the later paragraph; the test character lands in the heading (`## SectioXn 1`) |
| T1-18 | ISS-64 | Rendered ArrowDown does not enter the blockquote; typed `x` lands in the preceding paragraph |
| T1-19 | ISS-65 | One `x` typed in the Split preview code block also turns an unrelated blank line into a duplicate `# What to check`; first Ctrl+Z only undoes the stray heading |
| T1-20 | ISS-67 | Selection (3 lines) + Ctrl+click caret, then Tab: wrong lines indented / text moved into the wrong line (one Ctrl+Z does restore it) |

Good news since October: **no hang and no crash in any row of this re-run.** The ISS-1 deadlock (22 freezes in October) did not reproduce in 12 ISS-1 rows, and the ISS-2 `GDK has not been initialized` abort (exit 134) is gone. ISS-3, ISS-4, ISS-7 and most `5eec844` direct fixes and round-2 PRD fixes are confirmed (§3).

**Counts (98 rows, final results after supersession, §4):** **65 PASS / 23 FAIL / 7 PARTIAL / 2 BLOCKED / 1 DEFERRED** (0 SKIPPED rows; some sub-steps skipped, noted per row).

| Set | Rows | PASS | FAIL | PARTIAL | BLOCKED | DEFERRED |
|---|---:|---:|---:|---:|---:|---:|
| Tier 1 (rows 1–22 incl. 4b, 16b, 18b) | 25 | 20 | 5 | 0 | 0 | 0 |
| Round 1 blockers + supporting rows (non-Tier-1) | 27 | 7 | 14 | 4 | 2 | 0 |
| Direct fixes `5eec844` (non-Tier-1) | 16 | 15 | 1 | 0 | 0 | 0 |
| Round 2 PRD (non-Tier-1) | 24 | 19 | 1 | 3 | 0 | 1 |
| Tasks 28–32 spot-checks | 6 | 4 | 2 | 0 | 0 | 0 |
| **Total** | **98** | **65** | **23** | **7** | **2** | **1** |

Rows that appear in two tables of the doc (e.g. IMG-X1/X2/T1-14 under ISS-5 and ISS-46; SES-1/SES-3 under ISS-38/ISS-63; TBL-3 under ISS-61/ISS-62; T3-41 under ISS-35/ISS-41) are counted once. Tier 1 compared with October: 13 PASS / 11 FAIL / 1 BLOCKED → **20 PASS / 5 FAIL / 0 BLOCKED**.

## 1. Automated gate

| Check | Result | Numbers |
|---|---|---|
| Correct commit / clean tree | PASS | HEAD `9effd76` (code `221b6ae`), `git status --short` empty |
| `cargo build --release --locked` | PASS | `Finished release profile in 3m 38s`, exit 0. Evidence: `logs/rerun2-build.log`; rebuild after restart `logs/rerun2-rebuild.log` |
| `cargo test --locked` | PASS | **2228 passed / 0 failed / 2 ignored** (2230 tests, suite 1.75 s). Windows reference for this commit: **2239 passed** → Linux runs **11 fewer**. Same delta as in October (2067 vs 2078). Probably `#[cfg(windows)]` / platform-only tests (`path_utils`, `single_instance`, `fonts`, `terminal`, `lsp`); not verified test by test. Ignored: `editor::ferrite::buffer::tests::test_large_file`, `editor::ferrite::history::tests::test_large_buffer_performance`. Evidence: `logs/rerun2-cargo-test.log` |
| `cargo clippy --locked --release` | PASS (0 errors) | **0 errors, 321 warnings** for bin `ferrite` (198 auto-fixable), exit 0. Future-incompat notice for `ashpd v0.8.1`. Evidence: `logs/rerun2-clippy.log` |

Build note: the first build failed because `pkg-config` and the GTK/WebKit `-dev` packages were missing on the box. They were installed from Debian `.deb`s (the apt mirror returns HTTP 500 through the sandbox proxy). `libwebkit2gtk-4.1-dev` stayed half-configured (runtime vs `-dev` version skew), but `pkg-config` resolves `glib-2.0`, `gtk+-3.0`, `webkit2gtk-4.1` and `libsoup-3.0`, and the link succeeded. `cargo test` was not re-run after the restart rebuild (same commit, same toolchain).

## 2. Verdict against the doc's pass criteria

| Criterion (`rerun-v0.3.1-round2.md`) | Met? | Detail |
|---|---|---|
| 1. Every row in the tables is PASS or BLOCKED-by-input/environment | **No** | 23 FAIL, 7 PARTIAL, 1 DEFERRED (ISS-32, deferred to 0.3.2 by the doc itself, so not counted as a gate FAIL). The 2 BLOCKED rows (IMG-4, IMG-7) are blocked by the T1-14 product failure (ISS-66), **not** by input/environment, so they do not qualify either. |
| 2. Tier 1 rows 1–22 incl. 4b, 16b, 18b all PASS or BLOCKED-by-input/environment | **No** | 20 PASS, 5 FAIL: T1-14, T1-17, T1-18, T1-19, T1-20 |
| 3. Screenshots and `logs/ferrite-debug-*.log` attached for failures | Yes, with one gap | All FAIL/PARTIAL logs are in `rerun2-logs.zip`. Batch A (ISS-1 rows, 10:43–11:03) ran before `scrot` was installed; its screenshots were taken with the computer-use tool and are not saved as files on the box, so T1-17, T1-18, T2-35, ARR-2, ARR-4, IND-9 have logs but no screenshot file (see §7). |

Against the checklist's "Tag gate (minimal)": rule 1 ("All Tier 1 rows pass") fails on 5 rows, so **do not tag `v0.3.1`**. Suggested fix order before the next re-run: ISS-65 (stray heading written into the document + broken undo — file corruption), ISS-68 (document marked modified just by viewing it in Rendered), ISS-64 (Rendered arrow navigation), ISS-66 (X11 image paste + toast), ISS-67 (multi-cursor Tab), then the majors.

## 3. Per-ISS summary (every ISS in the re-run doc)

Status key: **Fixed** = every row for that ISS is PASS. **Fixed (follow-up)** = the doc's stated expectation is met, but a row is PARTIAL for a side aspect. **Partially fixed** = the original symptom is gone but the rows still FAIL or PARTIAL for the stated expectation. **Not fixed** = the stated expectation is not met.

### Round 1 blockers and supporting rows

| ISS | Fix commit | Rows → final | Status | Notes |
|---|---|---|---|---|
| ISS-1 | `5325e9b` | T1-2 PASS, T1-7 PASS, T1-17 FAIL, T1-18 FAIL, T1-18b PASS, ARR-1 PASS, ARR-2 FAIL, ARR-4 FAIL, CPY-3 FAIL, IND-9 FAIL, UX-9 PARTIAL, T2-35 FAIL | **Partially fixed** | The deadlock is gone: none of the 12 rows hung, the app stayed responsive and unsaved work was savable (T1-2, T1-7). The navigation/editing the rows exist to test still fails: ISS-64 (arrow nav), ISS-70 (CPY-3), ISS-71 (IND-9). |
| ISS-2 | `97347d3` | T2-36 FAIL, VID-1 FAIL, EXTRA-YT-2 FAIL, EXTRA-YT-4 FAIL, EXTRA-YT-6 PARTIAL, EXTRA-YT-7 PARTIAL, VID-10 PASS | **Partially fixed** | No `GDK has not been initialized` abort / exit 134 in any run (the stated expectation). But embeds never play (YouTube "Error 153"), offline shows black rectangles, `Gdk-CRITICAL _gdk_frame_clock_freeze/thaw` on the 12-embed page → ISS-69. |
| ISS-5 | `97347d3` | T1-14 FAIL, IMG-X1 FAIL, IMG-X2 FAIL, IMG-4 BLOCKED, IMG-7 BLOCKED | **Not fixed** | Clipboard holds `image/png` (verified via `TARGETS`); Ctrl+V inserts nothing; log `arboard … incorrect type received from clipboard` → ISS-66. |
| ISS-7 | `51f5a3a` | T1-21 PASS, MMD-12 PASS | **Fixed** | Opener typed char-by-char stays ```` ```mermaid ````, no `diamrem`, diagram renders. |
| ISS-9 | `51f5a3a` | EXTRA-SET-9a FAIL, EXTRA-SET-9b FAIL | **Partially fixed** | Corrupt file renamed to `config.json.corrupt-<ts>` (9a) and per-field tolerant load works (9b keeps other settings), but **no toast** in either case, and 9a silently starts from full defaults → ISS-72. |
| ISS-3 | `5325e9b` | T1-4b PASS, RLD-7 PASS | **Fixed** | UTF-16LE+BOM opens as text, saves/reloads as valid UTF-16LE CRLF. |
| ISS-4 | `5325e9b` | T1-15 PASS, SIF-1 PASS, SIF-2 PASS, SIF-3 PASS | **Fixed** | Caret lands on the match (CJK and ASCII, after `İ` too). |
| ISS-6 | `51f5a3a` | T1-19 FAIL, UND-1 FAIL | **Not fixed** | One preview code-block edit also writes a duplicate heading into an unrelated blank line; first Ctrl+Z only undoes that → ISS-65. |
| ISS-8 | `5325e9b` | T1-22 PASS, MMD-1 PARTIAL | **Fixed (follow-up)** | Popup opens, zoom/reset/close/scroll work, no hang. Hand mode does not pan while the diagram fits the viewport (pans once zoomed in). |

### Direct fixes (`5eec844`)

| ISS | Rows → final | Status | Notes |
|---|---|---|---|
| ISS-35 | T3-41 PASS | **Fixed** | Bold link text renders bold, no literal `**`. |
| ISS-37 | TTL-6b PASS, EXTRA-SET-1 PASS | **Fixed** | System title bar ON: Ctrl+, and the gear both open Settings. |
| ISS-38 | SES-1 PASS, SES-5 PASS, T3-46 PASS | **Fixed** | Exit dialog names the files and explains the Untitled/Quick note; note restored after relaunch. |
| ISS-43 | EXTRA-SET-5 PASS | **Fixed** | Auto-load Layout OFF: `.ferrite/terminal-layout.json` not loaded. |
| ISS-45 | PRI-3 PASS | **Fixed** | Middle-click closes the tab. |
| ISS-46 | IMG-X1 FAIL, IMG-X2 FAIL, T1-14 FAIL | **Not fixed** | No toast when the X11 clipboard read fails (the doc says the toast should appear even when the read fails) → part of ISS-66. |
| ISS-53 | UX-11 PASS | **Fixed** | MD quick reference lists Reload (Ctrl+Shift+R) and word wrap (Alt+Z). |
| ISS-55 | RUN-1 PASS | **Fixed** | Stdout from before Stop stays visible. |
| ISS-57 | MMD-3 FAIL | **Not fixed** | `linkStyle 0,1,2 interpolate basis` still draws straight segments → ISS-73. |
| ISS-58 | MMD-6 PASS | **Fixed** | Unknown `@pos` id: warning + diagram renders. |
| ISS-62 | TBL-3 PASS, T2-39 PASS | **Fixed** | Real Alt+Z toggles wrap without typing `z`. |
| ISS-63 | SES-6 PASS, SES-1 PASS, SES-3 PASS | **Fixed** | Window close dialog names `ses3-named.md`; app-exit dialog names multiple files. Observation (not filed): a second window holding only a dirty Untitled note did not close on its × or Alt+F4 and showed no prompt (seen once in Batch E, not retested). |

### Round 2 PRD (tasks 1–33)

| ISS | Task(s) | Rows → final | Status | Notes |
|---|---|---|---|---|
| ISS-36 | 1–4 | SES-6 PASS | **Fixed** | After restore, End + typing appends; save changed only line 10 (184 → 188 bytes), no "buffer shrank" log line. Minor note: one Ctrl+Z removed only the word ` W1` minus its leading space (word-granular undo), not data loss. |
| ISS-41 | 5–7 | TBL-2 PASS, EOL-3 PASS, EOL-5 PASS, T3-41 PASS | **Fixed** | Dirty `*` on the first key; saved files keep their EOLs and edits. |
| ISS-40 | 8–9 | UND-3 FAIL | **Not fixed** | Rendered code-block typing keeps only the first character; undo then leaves a stray `a` and a duplicate heading → ISS-65. |
| ISS-39 | 12 | CPY-2 PASS | **Fixed** | Copied code block byte-identical (`od -c`). First attempt gave an empty clipboard; second attempt with confirmed selection worked. |
| ISS-52 | 10–11 | UX-8 PASS | **Fixed** | Click in `tail` puts the caret in `tail`. Clicking the left edge lands one char early (edge note). |
| ISS-42 | 13 | UX-9 PARTIAL | **Partially fixed** | Plain click still edits; Ctrl+click is handled (`Opened link target via modifier-click`), but for a file target the box opens the **parent folder** listing in Chrome (no file manager installed) — matches "files reveal parent", not "opens target". |
| ISS-33 | 14 | T3-44 PARTIAL, HTML-6 PASS | **Fixed (follow-up)** | Adjacent single-line `<div align>` blocks now show their own text (the doc's expectation). Their alignment is not applied (both left-aligned) → T3-44 PARTIAL. HTML-6 re-graded, see §4.1. |
| ISS-44 | 19 | SIF-3 PASS | **Fixed** | `istanbul` matches `Istanbul`, `İstanbul`, `istanbul`. |
| ISS-47 | 20 | SPL-2 PASS, SPL-5 PASS | **Fixed** | `mistaek` flagged; code/URLs/camelCase/ALLCAPS not flagged. (A stale `mistaek` entry in `user-words.txt` had to be cleared first.) |
| ISS-48 | 21 | SPL-7 PASS | **Fixed** | Squiggles under the right words with wrap ON. |
| ISS-51 | 22 | IND-2 PASS | **Fixed** | Shift+Tab outdents a hard-tab line. |
| ISS-61 | 23 | TBL-3 PASS | **Fixed** | Wrap OFF guides align with body pipes. |
| ISS-56 | 24 | EXTRA-SET-7 PARTIAL | **Partially fixed** | All 10 locales listed. ja and zh-Hans Keyboard group headers (`File`, `Navigation`, `View`, `Edit`) and rows `Reload from Disk`, `Open Workspace`, `Close Workspace`, `New Window` still English; zh-Hans also leaves "Ferrite accent", "Reset color", "Rendered/Split/Raw" and the exit-dialog body in English. |
| ISS-30 | 25 | T2-32 PASS, EMO-1 PASS, EMO-2 PASS | **Fixed (documented limitation)** | Monochrome emoji, some ZWJ/flag blanks — documented, not a FAIL per the doc. |
| ISS-34 | 26 | SI-1 PASS, T2-37 PASS | **Fixed** | Covered primary window is raised and becomes active; forwarded open goes to the last-focused window. Note: a **minimized** primary got the tab but was not restored (WM vs app not determined). |
| ISS-54 | 27 | TTL-6a PARTIAL | **Partially verified** | Double-click maximize/restore works; drag-from-maximized could not be shown with synthetic drag (harness limit, not recorded as product FAIL). |
| ISS-49 | 18 | STAT-3 PASS | **By design (confirmed)** | Raw Enter does not continue list markers. |
| ISS-50 | 18 | EOL-9 PASS | **By design (confirmed)** | Mixed EOLs preserved per line; final unterminated line written with CRLF. |
| ISS-32 | 15 | T3-43 DEFERRED | **Deferred 0.3.2 (still present, no regression)** | No crash; subgraph TD arrow crosses "Inner Group" title, nested subgraphs drawn as siblings; git-graph labels ellipsised. |
| ISS-59 | 17 | MMD-13 PASS | **Fixed** | CJK node boxes hug labels (judged by eye, no pixel measurement). |
| ISS-60 | 16 | MMD-10 PASS | **Fixed** | `autonumber off` after msg 4 → badges 1–4 only; `autonumber 10 10` → 10, 20, 30…; badges by the arrows, not on text. Alt-frame right edge clipped (cosmetic). |

### Tasks 28–32 spot-checks (no ISS id)

| Observation | Task | Row → final | Notes |
|---|---|---|---|
| SI-3 | 28 | PASS | `--new-instance` did not change `instance.pid` or `session.json` (md5 unchanged). The "does not clear primary crash recovery" half was not exercised (recovery dir empty at the time). |
| SI-4 | 29 | **FAIL** → ISS-74 | Settings toggle not written to `config.json` within ~500 ms (nor 1–5 s); only persisted on graceful exit. |
| GIT-3 | 30 | PASS | `Ignoring file watcher event for own save`; no reload prompt. |
| CSV-4 | 31 | PASS | No "Showing raw content" banner (2.1 MB grid and 85 MB Raw). |
| SPL-3 | 31 | **FAIL** → ISS-75 | Spell hint tooltip drawn on top of the open context menu. |
| HTML-4 | 32 | PASS | `## Line one<br>line two` and `<kbd>` in headings render their text (kbd has no chip styling inside headings). |

**Confirmed fixed (all rows PASS, or stated expectation met):** ISS-3, ISS-4, ISS-7, ISS-8 (follow-up), ISS-33 (follow-up), ISS-34, ISS-35, ISS-36, ISS-37, ISS-38, ISS-39, ISS-41, ISS-43, ISS-44, ISS-45, ISS-47, ISS-48, ISS-51, ISS-52, ISS-53, ISS-55, ISS-58, ISS-59, ISS-60, ISS-61, ISS-62, ISS-63; plus ISS-30 (documented), ISS-49/ISS-50 (by design). **Partially fixed:** ISS-1, ISS-2, ISS-9, ISS-42, ISS-56, ISS-54 (partially verified). **Not fixed:** ISS-5, ISS-6, ISS-40, ISS-46, ISS-57. **Deferred:** ISS-32.

## 4. Full row table (final results)

Supersession rules applied (later re-check wins):
- **CPY-3:** Batch A's FAIL is invalid (`xclip` was missing, so the clipboard could not be read); Batch B's redo is authoritative (FAIL, different reason).
- **T1-2:** Batch A PARTIAL (Ctrl+S did not clear `*`) superseded by Batch D's Ctrl+S re-check PASS. Combined T1-2 = PASS (click-away commit without hang, Alt+F4 → Save, CRLF 8/8 in A; Ctrl+S clears `*` and keeps CRLF in D). Batch D's remark "prior full-row hang remains" refers to the October run and is ignored.
- **T1-4, T1-5, T1-6, T1-8, T1-16:** Batch D's first-pass FAILs were harness artifacts (no confirmed keyboard focus in the text area; Aa toggle click landed on the tab strip). Config had no shortcut overrides (`keyboard_shortcuts.bindings: {}`). Focus-confirmed re-runs (`RERUN-D2-*`) are PASS.
- **T1-21:** Batch C's retry PASS supersedes Batch B's PARTIAL (xdotool dropped a CJK character from the malformed sample). MMD-12 PASS (B).
- **SES-3:** Batch E's BLOCKED (second window had only an Untitled note) superseded by Batch H's PASS with a named file.
- **MMD-10:** Batch G's PARTIAL (variants missing from fixture) superseded by Batch H's PASS (variants built in `mmd10-sequence-variants.md`).
- **UX-9:** Batch A (FAIL) superseded by Batch F (PARTIAL, later, on the intended `test_local_links.md`).
- **TBL-3:** Batch E (Alt+Z half) + Batch G (guides half) both PASS → PASS.
- **HTML-6:** re-graded, see §4.1.

Log names are abbreviated `…HHMMSS-tag.log` = `logs/ferrite-debug-20261109-HHMMSS-tag.log`. "no file" = the screenshot was a live computer-use capture not saved on the box.

### 4.1 HTML-6 re-grade

Batch F reported that switching to Rendered rewrote line 1 to `A# GitHub HTML Subset Test`. **That was wrong:** the committed fixture `test_md/test_github_html.md` already starts with `A#` (since commit `570264b`; `git status` clean, Batch H confirmed). What Batch F actually saw was a spurious dirty `*` plus 3 × `Content modified in rendered editor, recorded for undo` right after `Set view mode to Rendered` with no input (`…132337-R2-HTML-6.log` lines 3055–3131). That behaviour is filed as **ISS-68** (not as HTML-6).

HTML-6's own expectation is "Single-line `<div align>…</div>` coalesces" (total-test §7) / "Adjacent single-line `<div align>` blocks show **their** text, not first document line" (re-run doc, ISS-33). In the same run on the same fixture section ("Adjacent divs"), Batch F's T3-44 observation shows "First centered block." and "Second block, right-aligned." rendering as their own text. The October failure mode (first document line drawn in their place) is gone. **HTML-6 = PASS.** Alignment not being applied is graded under T3-44 (PARTIAL), and the dirty flag under ISS-68.

**Note for the maintainer:** line 1 of `test_md/test_github_html.md` reads `A# GitHub HTML Subset Test` — the leading `A` looks like a typo, so the fixture's title does not render as a heading.

### 4.2 Table

| Row | Set / ISS | Final | Batch | Evidence (in zips) | Notes |
|---|---|---|---|---|---|
| T1-1 | Tier 1 | PASS | D | …120916-R2-T1-1.log; shots/R2-T1-1-pass.png | No dirty `*` after click + Escape |
| T1-2 | Tier 1 / ISS-1 | PASS | A + D | …104340-T1-2.log, …121014-R2-T1-2.log, logs/R2-T1-2-eol-check.txt; shots/R2-T1-2-pass.png, rerun2/T1-2-startup.webp | Click-away commit, no hang; Ctrl+S clears `*`; CRLF 8/8 |
| T1-3 | Tier 1 | PASS | D | …121014-R2-T1-2.log, logs/R2-T1-3-check.txt; shots/R2-T1-3-pass.png | CRLF 8, lone LF 0 |
| T1-4 | Tier 1 | PASS | D (re-run) | …123435-RERUN-D2-T1-4.log, logs/RERUN-D2-T1-4-eol-check.txt; shots/RERUN-D2-T1-4-pass.png | Dup line + TOC; CRLF 15/15 |
| T1-4b | Tier 1 / ISS-3 | PASS | C | …112724-R2-T1-4b.log, logs/R2-T1-4b-check.txt; shots/R2-T1-4b-pass.png | Text renders faint grey (cosmetic, not investigated) |
| T1-5 | Tier 1 | PASS | D (re-run) | logs/RERUN-D2-T1-5-log-tail.txt | Clean tab reloads `FIRST` |
| T1-6 | Tier 1 | PASS | D (re-run) | logs/RERUN-D2-T1-6-log-tail.txt; shots/RERUN-D2-T1-6-pass.png | Dialog; Cancel keeps edit; Don't Save loads disk |
| T1-7 | Tier 1 / ISS-1 | PASS | A | …104547-T1-7.log; no file | Dialog, Cancel keeps edit, no hang |
| T1-8 | Tier 1 | PASS | D (re-run) | logs/RERUN-D2-T1-8-log-tail.txt; shots/RERUN-D2-T1-8-dialog-dismissed.png | Caveat: closing the other tab also dismissed the reload dialog; re-triggered; other file untouched |
| T1-9 | Tier 1 | PASS | D | …121419-R2-T1-9.log | Alt+F4 → Save saves both |
| T1-10 | Tier 1 | PASS | D | …121537-R2-T1-10.log, …121616-R2-T1-10b.log; shots/R2-T1-10-pass.png | |
| T1-11 | Tier 1 | PASS | D | …121644-R2-T1-11.log, …121718-R2-T1-11-relaunch.log, logs/R2-T1-11-disk-check.txt; shots/R2-T1-11-pass.png | Kill -9 → recovery → one Ctrl+Z |
| T1-12 | Tier 1 | PASS | D | …121758-R2-T1-12-13.log; shots/R2-T1-12-pass.png | |
| T1-13 | Tier 1 | PASS | D | …121758-R2-T1-12-13.log; shots/R2-T1-13-pass.png | Status-bar line count stale during sequence |
| T1-14 | Tier 1 / ISS-5, ISS-46 | **FAIL** | B | …111926-R2-T1-14.log; shots/R2-T1-14-fail.png | ISS-66 |
| T1-15 | Tier 1 / ISS-4 | PASS | C | …112856-R2-T1-15.log; shots/R2-T1-15-pass.png | Ln 1, Col 6 |
| T1-16 | Tier 1 | PASS | D (re-run) | logs/RERUN-D2-T1-16-log-tail.txt; shots/RERUN-D2-T1-16-pass.png, -controls-zoom.png, -match-zoom.png | |
| T1-16b | Tier 1 | PASS | D | …122517-R2-T1-16.log; shots/R2-T1-16b-pass.png | |
| T1-17 | Tier 1 / ISS-1 | **FAIL** | A | …104703-T1-17.log; no file | ISS-64 (no hang) |
| T1-18 | Tier 1 / ISS-1 | **FAIL** | A | …105042-T1-18.log; no file | ISS-64 (no hang) |
| T1-18b | Tier 1 / ISS-1 | PASS | A | …105213-T1-18b.log; no file | Prefixes kept |
| T1-19 | Tier 1 / ISS-6 | **FAIL** | C | …115009-R2-T1-19.log (+ pre-restart …113156-R2-T1-19.log); shots/R2-T1-19-fail.png, R2-T1-19-before-undo.png | ISS-65 |
| T1-20 | Tier 1 | **FAIL** | D | …122703-R2-T1-20.log, logs/R2-T1-20-buffer-before-undo.txt; shots/R2-T1-20-fail.png | ISS-67 |
| T1-21 | Tier 1 / ISS-7 | PASS | C (retry) | …115953-R2-T1-21.log (B: …112248-R2-T1-21.log); shots/R2-T1-21-partial.png (B attempt) | |
| T1-22 | Tier 1 / ISS-8 | PASS | C | …115239-R2-T1-22.log; shots/R2-T1-22-pass.png | |
| ARR-1 | ISS-1 | PASS | A | …105645-ARR-1.log; no file | Scrolls, no hang |
| ARR-2 | ISS-1 | **FAIL** | A | …105726-ARR-2.log; no file | ISS-64 |
| ARR-4 | ISS-1 | **FAIL** | A | …105807-ARR-4.log; no file | ISS-64 |
| CPY-3 | ISS-1 | **FAIL** | B (redo) | …110716-R2-CPY-3.log; shots/R2-CPY-3-fail.png | ISS-70 |
| IND-9 | ISS-1 | **FAIL** | A | …110047-IND-9.log; no file | ISS-71 |
| UX-9 | ISS-1, ISS-42 | PARTIAL | A + F | …110140-UX-9.log, …132424-R2-UX-9.log; rerun2/R2-UX-9-chrome-dir-index.webp | Reveals parent folder (Chrome dir index; no file manager) |
| T2-35 | ISS-1 (ISS-31) | **FAIL** | A | …105458-T2-35.log; no file | ISS-64; restart/persistence step not reached |
| T2-36 | ISS-2 | **FAIL** | B | …110845-R2-T2-36.log; shots/R2-T2-36-fail.png, R2-T2-36-pass.png | ISS-69 |
| VID-1 | ISS-2 | **FAIL** | B | …110956-R2-VID-1.log; shots/R2-VID-1-fail.png | ISS-69 |
| EXTRA-YT-2 | ISS-2 | **FAIL** | B | …111035-R2-EXTRA-YT-2.log; shots/R2-EXTRA-YT-2-fail.png | ISS-69 (offline) |
| EXTRA-YT-4 | ISS-2 | **FAIL** | B | …111123-R2-EXTRA-YT-4.log; shots/R2-EXTRA-YT-4-fail.png | ISS-69; RSS 286 380 KB |
| EXTRA-YT-6 | ISS-2 | PARTIAL | B | …111221-R2-EXTRA-YT-6.log, …111333-R2-EXTRA-YT-6b.log; shots/R2-EXTRA-YT-6-partial.png | No crash; live URL edit not verifiable (xdotool appended instead of replacing) |
| EXTRA-YT-7 | ISS-2 | PARTIAL | B | …111452-R2-EXTRA-YT-7.log; shots/R2-EXTRA-YT-7-partial.png, -pass-search.png, -pass-reload.png | Reload + Search dialogs draw above players; Settings opens as tab, playback unverifiable |
| VID-10 | ISS-2 | PASS | B | T2-36/VID-1 logs; `ldd` in B results | No GDK abort |
| IMG-X1 | ISS-5, ISS-46 | **FAIL** | B | …111926-R2-T1-14.log; shots/R2-IMG-X1-fail.png | ISS-66 |
| IMG-X2 | ISS-5, ISS-46 | **FAIL** | B | …111926-R2-T1-14.log; shots/R2-IMG-X2-fail.png | ISS-66 |
| IMG-4 | ISS-5 | BLOCKED (by ISS-66) | B | …111926-R2-T1-14.log; shots/R2-T1-14-fail.png | Needs a working paste first; blocked by a product defect, not environment |
| IMG-7 | ISS-5 | BLOCKED (by ISS-66) | B | …111926-R2-T1-14.log; shots/R2-IMG-7-blocked.png | Same |
| MMD-12 | ISS-7 | PASS | B | …112248-R2-T1-21.log; shots/R2-MMD-12-pass.png | |
| EXTRA-SET-9a | ISS-9 | **FAIL** | C | …115756-R2-EXTRA-SET-9a.log; shots/R2-EXTRA-SET-9a-fail.png | ISS-72 |
| EXTRA-SET-9b | ISS-9 | **FAIL** | C | …115843-R2-EXTRA-SET-9b.log; shots/R2-EXTRA-SET-9b-fail.png | ISS-72 |
| RLD-7 | ISS-3 | PASS | C | …112724-R2-T1-4b.log, logs/R2-RLD-7-utf16-check.txt; shots/R2-RLD-7-pass.png | UTF-16 half only (cp1252 half passed in October) |
| SIF-1 | ISS-4 | PASS | C | …112856-R2-T1-15.log; shots/R2-SIF-1-pass.png | |
| SIF-2 | ISS-4 | PASS | C | same; shots/R2-SIF-2-pass.png | Col 9 |
| SIF-3 | ISS-4, ISS-44 | PASS | C | same; shots/R2-SIF-3-pass.png | 3 matches incl. `İstanbul` |
| UND-1 | ISS-6 | **FAIL** | C | …115152-R2-UND-1.log; shots/R2-UND-1-fail.png | ISS-65 |
| MMD-1 | ISS-8 | PARTIAL | C | …115441-R2-MMD-1.log; shots/R2-MMD-1-partial.png | Hand pans only when zoomed past the viewport |
| T3-41 | ISS-35, ISS-41 | PASS | E | …125315-R2-T3-41.log | |
| TTL-6b | ISS-37 | PASS | E | …125725-R2-TTL-6b.log | |
| EXTRA-SET-1 | ISS-37 | PASS | E | …125725-R2-TTL-6b.log | `shots/R2-EXTRA-SET-1-blocked.png` (12:59) is an intermediate Batch E capture; E recorded the row as PASS |
| SES-1 | ISS-38, ISS-63 | PASS | E | …125725-R2-TTL-6b.log, …130614-R2-SES-5.log; shots/R2-SES-1-pass.png | |
| SES-5 | ISS-38 | PASS | E | …130614-R2-SES-5.log | |
| T3-46 | ISS-38 | PASS | E | …125725-R2-TTL-6b.log; shots/R2-T3-46-pass.png | |
| EXTRA-SET-5 | ISS-43 | PASS | E | …125725-R2-TTL-6b.log; shots/R2-EXTRA-SET-5-pass.png | |
| PRI-3 | ISS-45 | PASS | E | …125725-R2-TTL-6b.log; shots/R2-PRI-3-pass.png | |
| UX-11 | ISS-53 | PASS | E | shots/R2-UX-11-pass.png | |
| RUN-1 | ISS-55 | PASS | E | shots/R2-RUN-1-pass.png | |
| MMD-3 | ISS-57 | **FAIL** | E | …115441-R2-MMD-1.log, logs/R2-MMD-3-log-tail.txt; shots/R2-MMD-3-fail.png | ISS-73 |
| MMD-6 | ISS-58 | PASS | E | …125315-R2-T3-41.log; shots/R2-MMD-6-pass.png | |
| TBL-3 | ISS-61, ISS-62 | PASS | E + G | …125725-R2-TTL-6b.log, …132923-R2-TBL-3.log, …132951-R2-TBL-3b.log; shots/R2-TBL-3-wrapoff.png | |
| T2-39 | ISS-62 | PASS | E | …125725-R2-TTL-6b.log | Real Alt+Z |
| SES-3 | ISS-63 | PASS | H | …134343-R2-GIT-3.log, …134720-R2-SES-3-fwd.log; shots/R2-SES-3-dialog-pass.webp (E: shots/R2-SES-3-blocked.png, R2-SES-3-no-dialog.png) | Untitled-only window observation, §6 |
| SES-6 | ISS-36, ISS-63 | PASS | H | …135010-R2-SES6.log, …134943-R2-SES6-fwd.log | Undo granularity note |
| TBL-2 | ISS-41 | PASS | F | F1–F4 logs | |
| EOL-3 | ISS-41 | PASS | F | F1–F4 logs | CRLF kept |
| EOL-5 | ISS-41 | PASS | F | F1–F4 logs | LF kept |
| UND-3 | ISS-40 | **FAIL** | F | logs/R2-UND-3-log-tail.txt, F1–F4 logs, rerun2/R2-UND-3-diff.txt, rerun2/R2-UND-3-corrupted-file.md; shots/R2-UND-3-fail-attempt1.png | ISS-65 |
| CPY-2 | ISS-39 | PASS | F | F1–F4 logs | Byte-identical on 2nd attempt |
| UX-8 | ISS-52 | PASS | F | F1–F4 logs | |
| T3-44 | ISS-33 | PARTIAL | F | …132337-R2-HTML-6.log; rerun2/R2-HTML-6-spurious-dirty.webp | Own text OK; alignment not applied |
| HTML-6 | ISS-33 | PASS (re-graded) | F | …132337-R2-HTML-6.log; rerun2/R2-HTML-6-spurious-dirty.webp | See §4.1; dirty flag → ISS-68 |
| SPL-2 | ISS-47 | PASS | G | …132614-R2-SPL-2.log, …132656-R2-SPL-2b.log; shots/R2-SPL-2-pass.png | |
| SPL-5 | ISS-47 | PASS | G | same | |
| SPL-7 | ISS-48 | PASS | G | same | |
| IND-2 | ISS-51 | PASS | G | …132835-R2-IND-2.log; shots/R2-IND-2-pass.png | |
| EXTRA-SET-7 | ISS-56 | PARTIAL | G + H | …133100-R2-EXTRA-SET-7.log, …134343-R2-GIT-3.log; shots/R2-EXTRA-SET-7-ja-keyboard.webp, -zh-keyboard.webp, R2-EXTRA-SET-7-partial.png | Untranslated labels |
| T2-32 | ISS-30 | PASS (documented) | G | …133236-R2-T2-32.log | |
| EMO-1 | ISS-30 | PASS (documented) | G | same; shots/R2-EMO-1-raw.png | |
| EMO-2 | ISS-30 | PASS (documented) | G | same; shots/R2-EMO-2-rendered.png | |
| SI-1 | ISS-34 | PASS | H | …134101-R2-ISS34.log, …134115-…fwd.log, …134131-…fwd2.log, …134144-R2-SI1-time.log; shots/R2-ISS34-pass.png | 2nd process exits in 0.14 s; minimized primary not restored (note) |
| T2-37 | ISS-34 | PASS | H | …134205-R2-T2-37-fwd.log | "Play video in window 1" sub-step skipped (YouTube unplayable here, ISS-69) |
| TTL-6a | ISS-54 | PARTIAL | G | …133348-R2-TTL-6a.log; shots/R2-TTL-6a-partial.png | Drag-to-restore untestable with synthetic drag |
| STAT-3 | ISS-49 | PASS (by design) | G | …133509-R2-STAT-3.log | |
| EOL-9 | ISS-50 | PASS (by design) | G | …133656-R2-EOL-9.log, logs/R2-EOL-9-check.txt | |
| T3-43 | ISS-32 | DEFERRED (0.3.2) | G | …133805-R2-MMD-10.log; rerun2/shots-G/R2-T3-43-subgraph-td-deferred.webp | Still known, no regression |
| MMD-13 | ISS-59 | PASS | G | …133805-R2-MMD-10.log; rerun2/shots-G/R2-MMD-13-cjk-flowchart.webp | By eye |
| MMD-10 | ISS-60 | PASS | H (G partial) | …134917-R2-MMD-10.log, …134343-R2-GIT-3.log, rerun2/mmd10-sequence-variants.md; shots/R2-MMD-10-autonumber.png, rerun2/shots-G/R2-MMD-10-autonumber.webp | Alt frame clipped right (cosmetic) |
| SI-3 | task 28 | PASS | H | …134219-R2-SI-3.log | Recovery half not exercised |
| SI-4 | task 29 | **FAIL** | H | …134250-R2-SI-4.log, …134308-R2-SI-4b.log; shots/R2-SI-4-fail.webp | ISS-74 |
| GIT-3 | task 30 | PASS | H | …134343-R2-GIT-3.log | |
| CSV-4 | task 31 | PASS | H | …134343-R2-GIT-3.log; shots/R2-CSV-4-pass.webp | |
| SPL-3 | task 31 | **FAIL** | H | …134343-R2-GIT-3.log, …134553-R2-SPL-3.log; shots/R2-SPL-3-fail.webp | ISS-75 |
| HTML-4 | task 32 | PASS | H | …134343-R2-GIT-3.log; shots/R2-HTML-4-pass.webp | kbd no chip in headings |

## 5. New issues (ISS-64 …)

The highest ID in the October report and the re-run doc is ISS-63 (checked with `grep -oE 'ISS-[0-9]+'` on both files), so new issues start at **ISS-64**. Every FAIL row above maps to exactly one new issue; rows with a shared root cause are grouped. Nothing was filed on GitHub and the round-2 PRD was not edited. Severity follows the brief: Tier 1 FAIL = **blocker**; otherwise crash/hang/data loss/file corruption = blocker, feature not working as described = major, wrong but workable = minor, cosmetic = polish. "Clean exit" = launcher footer `Ferrite exited with code 0`.

> **Status column** (added 2026-11-09 by the round-3 triage; GrokBot's text is untouched): *fixed (working tree)* = code + regression test in the round-3 fix commit, GrokBot re-run pending per `rerun-v0.3.1-round3.md`; *task N* = round-3 PRD orchestrator task; *by design* / *deferred* = decision in `docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md` §8. A FAIL row is only closed by code or a §8 decision — never by editing the row.

| ID | Title | Severity | Rows | Status |
|---|---|---|---|---|
| ISS-64 | Rendered/preview arrow navigation does not move the caret between blocks (no hang any more) | **blocker** (Tier 1) | T1-17, T1-18, ARR-2, ARR-4, T2-35 | **fixed (working tree)** — scope-stable `BlockRef::widget_id` (`rendered_session.rs`); successor of ISS-1/ISS-31, round-2 fix never worked end-to-end — re-run pending |
| ISS-65 | Code-block edit in Split preview / Rendered writes a duplicate heading into an unrelated blank line; undo is split; Rendered drops typed characters | **blocker** (Tier 1; file corruption) | T1-19, UND-1, UND-3 | **fixed (working tree)** — non-dirty cold seed + active-only live write + stale-heading guard (`editor.rs`); round-2 regression (`221b6ae` write-through × dirty heading seed) — re-run pending |
| ISS-66 | X11: Ctrl+V of a PNG inserts nothing (egui/arboard "incorrect type received from clipboard"), and no failure toast | **blocker** (Tier 1) | T1-14, IMG-X1, IMG-X2 (blocks IMG-4, IMG-7) | **Linux-verify-only** — `debug!` on every silent exit of `try_consume_clipboard_image_paste` (working tree); re-run with `--log-level debug` decides PRD task 2 |
| ISS-67 | Multi-cursor Tab (3-line selection + Ctrl+click caret) indents the wrong text | **blocker** (Tier 1) | T1-20 | **fixed (working tree)** — any non-empty range is a line indent (`is_line_indent_range`); log shows the selection was single-line `(6,0)–(6,14)`; line-33 mid-caret insert is by design — re-run pending (severity → major) |
| ISS-68 | Switching a clean document to Rendered marks it modified (`*`) with no user input | **major** (possible silent content change; see notes) | HTML-6 repro (row re-graded PASS), plus Batch G/H reproductions; no row of its own | **fixed (working tree)** — same change set as ISS-65; confirmed shared cause (dirty heading seed rewritten by live frame, `heading_plain_text` strips backticks/HTML → 6376→6374) — severity → blocker; re-run pending |
| ISS-69 | Linux YouTube embeds never play (YouTube "Error 153"); offline shows black rectangles; `Gdk-CRITICAL` frame-clock warnings | **major** | T2-36, VID-1, EXTRA-YT-2, EXTRA-YT-4 (also EXTRA-YT-6/7 PARTIAL) | **open — PRD task 1** (Linux-gated): `ferrite-video://` custom scheme is a non-HTTP origin on webkit2gtk → no Referer → Error 153; Windows maps it to `https://ferrite-video.localhost` |
| ISS-70 | Copy from a Rendered inline-formatted paragraph puts raw Markdown on the clipboard | **major** | CPY-3 | **by design (0.3.1)** — click-to-edit opens the source TextEdit; "copy as plain text" → 0.3.2 (task 4). Row stays FAIL vs its expected text; BLOCKED-by-decision in round 3 |
| ISS-71 | Tab on a Rendered list item does nothing | **major** | IND-9 | **deferred 0.3.2 (feature)** — never implemented (`check_structural_keys`/`indent_list_item` unwired); round-2 expected text assumed a non-existent feature; BLOCKED-by-decision in round 3 |
| ISS-72 | Corrupt / invalid config loads with no user-visible toast; invalid JSON silently falls back to full defaults | **major** | EXTRA-SET-9a, EXTRA-SET-9b | **fixed (working tree)** — toast fired (log shows the rename) but lived 5 s vs screenshot ~1 min later; now 30 s + `info!("Config load toast: …")` — re-run with screenshot ≤ 20 s (severity → minor) |
| ISS-73 | Indexed `linkStyle 0,1,2 interpolate basis` still draws straight edges | **minor** | MMD-3 | **by design (fixture)** — collinear `A→B→C→D` chain is straight under any spline (Mermaid.js too); parser + Catmull-Rom verified; fixture §3 replaced with fan-out/fan-in — re-run on new fixture |
| ISS-74 | Settings toggles are not persisted until graceful exit (lost on kill) | **major** | SI-4 | **fixed (working tree)** — debounce starved by per-frame `mark_settings_dirty` from `update_window_geometry_for` + two clocks; now first-change anchor, change-only geometry dirtying, single `settings_mono_ms` — re-run pending |
| ISS-75 | Spell-check hint tooltip drawn on top of the open context menu | **polish** | SPL-3 | **fixed (working tree)** — `Context::any_popup_open` only sees same-pass popups shown before the tooltip; now also memory-tracked + previous-frame state — re-run pending |

### ISS-64 — Rendered/preview arrow navigation does not move the caret between blocks
- **Rows:** T1-17, T1-18 (Tier 1), ARR-2, ARR-4, T2-35. Severity **blocker** (Tier 1). Successor to the navigation half of ISS-1 / ISS-31; the deadlock itself is fixed (no hang in any row, app responsive, clean exits).
- **Repro (T1-17):** 1. Copy `test_md/test_find_arrow_nav.md` to a temp folder and open it. 2. **Rendered**. 3. Click the first heading (`## Section 1`). 4. Press **ArrowDown** ~15 times. 5. Type `X`. 6. Switch to **Raw**.
- **Expected:** the caret walks down block by block and reaches the off-screen multi-line paragraph; the `X` lands there; the paragraph stays complete.
- **Actual:** navigation never reaches the later paragraph; the `X` lands in the heading: `## SectioXn 1`. Ctrl+Z restores it.
- **Other reproductions:** T1-18 — ArrowDown from the paragraph before the blockquote never enters the quote; `x` lands in the preceding paragraph (quote intact in Raw). ARR-2 — click the 2nd visual row of the wrapped Section 4 paragraph, ArrowDown: page scrolls, but no caret / visual-row movement, status bar stays `Ln 1, Col 1`. ARR-4 — Split, Raw ArrowDown moves Ln 10 → 11 (OK), but ArrowDown in the focused Rendered paragraph does nothing visible (Raw focus is not stolen). T2-35 — Split with preview lock ON: Raw arrows and edits work, Rendered ArrowDown does nothing (row 35 expects "arrow navigation still moves"); typing `Q` correctly does nothing; the restart/persistence step was not reached. ARR-1 (ArrowDown ×2 from a Section 1 paragraph) did scroll to the following blocks and passed.
- **Evidence:** `logs/ferrite-debug-20261109-104703-T1-17.log`, `…105042-T1-18.log`, `…105726-ARR-2.log`, `…105807-ARR-4.log`, `…105458-T2-35.log`. Log tails (last ~30 non-span lines, in `rerun2/results-A-iss1.md`) show only normal shutdown, no WARN/ERROR. Screenshots were live computer-use captures not saved as files (Batch A ran before `scrot` was reinstalled).

### ISS-65 — Code-block edit in Split preview / Rendered corrupts an unrelated line; undo split; dropped characters
- **Rows:** T1-19 (Tier 1), UND-1 (successor to ISS-6), UND-3 (successor to ISS-40). Severity **blocker** (Tier 1, and the document content is changed in a place the user did not edit).
- **Repro (T1-19 / UND-1):** 1. Copy `test_md/test_copy_and_split_undo.md` to `/tmp`. 2. Open it, **Split**. 3. In the preview, click at the end of the Python code block and type one `x`. 4. Inspect Raw. 5. Press **Ctrl+Z** once.
- **Expected:** only `x` is added (line 43); one Ctrl+Z removes it.
- **Actual:** `x` appears at line 43, **and** the unrelated blank line 49 becomes a duplicate `# What to check`. The first Ctrl+Z removes only the stray heading; `x` stays; a second Ctrl+Z removes `x`. Three Ctrl+Y/Ctrl+Z cycles each re-add/remove only the stray heading. SENTINEL lines stay intact. Reproduced on a fresh build (after the box restart) in both rows.
- **Repro (UND-3):** same fixture in **Rendered** (not Split): click into the Python code block and type several characters; then Ctrl+Z.
- **Actual (UND-3):** only the first typed character is kept, the rest are dropped (retried slower and after refocusing). After undo the document differs from the original: `diff` = `42a43 > a` and `48c49 < (blank) > # What to check` — the same stray heading as T1-19 plus a leftover `a`.
- **Evidence:** `logs/ferrite-debug-20261109-115009-R2-T1-19.log` (undo/redo lines `Undo performed, 1 entries remaining` / `Redo performed, 0 entries remaining` ×3 at 11:50:53–11:51:28 CET), `…113156-R2-T1-19.log` (pre-restart attempt), `…115152-R2-UND-1.log`, `logs/R2-UND-3-log-tail.txt`, `logs/ferrite-debug-20261109-1315…1321-R2-F1…F4.log`, `rerun2/R2-UND-3-diff.txt`, `rerun2/R2-UND-3-corrupted-file.md`; `shots/R2-T1-19-fail.png`, `shots/R2-T1-19-before-undo.png`, `shots/R2-UND-1-fail.png`, `shots/R2-UND-3-fail-attempt1.png`. All runs exited cleanly.

### ISS-66 — X11 image paste: Ctrl+V of a PNG inserts nothing and shows no toast
- **Rows:** T1-14 (Tier 1), IMG-X1, IMG-X2; blocks IMG-4 and IMG-7. Successor to ISS-5 and ISS-46. Severity **blocker** (Tier 1).
- **Repro:** 1. `convert -size 64x64 xc:red /tmp/red.png` 2. `DISPLAY=:2 xclip -selection clipboard -t image/png -i /tmp/red.png`; check `xclip -selection clipboard -o -t TARGETS` lists `image/png`. 3. Open a **saved** `.md` (e.g. `/tmp/ferrite-rerun2/paste.md`), click at the end of a line, press **Ctrl+V**. 4. Repeat in a new **Untitled** tab (IMG-X1) and twice quickly in the saved file (IMG-X2).
- **Expected:** `![](assets/…png)` inserted at the caret and the PNG written under `assets/` next to the file; in an Untitled tab a "save the document first" toast; on any failure to read the clipboard, a toast (ISS-46).
- **Actual:** nothing is inserted, no `assets/` folder is created, and no toast appears in any of the three cases. Each Ctrl+V logs `ERROR egui_winit::clipboard] arboard paste error: Unknown error while interacting with the clipboard: incorrect type received from clipboard` (5 times, 11:19:51–11:22:15 CET). The error comes from egui-winit's own paste handling, so Ferrite's image-paste handler and its failure toast are apparently never reached. Same error as in October.
- **Caveat:** the clipboard owner was `xclip` offering only `image/png` (no screenshot tool on the box). A real screenshot tool may offer other targets; please re-check on a real X11 desktop. The missing toast does not depend on this.
- **Evidence:** `logs/ferrite-debug-20261109-111926-R2-T1-14.log`; `shots/R2-T1-14-fail.png`, `shots/R2-IMG-X1-fail.png`, `shots/R2-IMG-X2-fail.png`, `shots/R2-IMG-7-blocked.png`.

### ISS-67 — Multi-cursor Tab indents the wrong text
- **Rows:** T1-20 (Tier 1; BLOCKED in October because Alt+click is grabbed by xfwm4 — this build uses Ctrl+click, which reaches the app). Severity **blocker** (Tier 1).
- **Repro:** 1. Open a copy of `test_indent_selection.md`, Raw. 2. Select Block 1 lines 7–9. 3. **Ctrl+click** on line 33 to add a caret. 4. Press **Tab**. 5. Press Ctrl+Z once.
- **Expected:** lines 7, 8, 9 and 33 each indented by one level; nothing moved into another line; one Ctrl+Z reverts the whole indent.
- **Actual:** line 7 was replaced by four spaces, lines 8–9 were unchanged, and line 33 became `中文行    首` (the indent was inserted mid-line). One Ctrl+Z did restore the file byte-for-byte (that part passes). Log: `EditorWidget: cursor sync changed: (6, 14) -> (6, 4)` then `Content modified in raw editor`.
- **Evidence:** `logs/ferrite-debug-20261109-122703-R2-T1-20.log`, `logs/R2-T1-20-buffer-before-undo.txt`; `shots/R2-T1-20-fail.png`.

### ISS-68 — Switching a clean document to Rendered marks it modified with no user input
- **Rows:** none failed on it directly; found while running HTML-6 (F), T3-43/MMD rows (G) and HTML-4 (H). Severity **major**: the tab shows `*`, the log records an edit, and the exit dialog then asks to save a file the user never touched; if the user saves, Ferrite writes whatever the Rendered pass produced. Content change was **not byte-verified** because nothing was saved, but in Batch G the character count of a copy of `test_mermaid_issue_165.md` changed 6376 → 6374 on entering Rendered, so a real content change is likely (would make this a blocker).
- **Repro:** 1. Open a clean copy of `test_md/test_github_html.md` (Raw). 2. Click **Rendered**. Do not type or click in the document.
- **Expected:** tab stays clean; no "Content modified" log line.
- **Actual:** tab title gets `*` immediately; the log shows `Title bar: Set view mode to Rendered` followed by 3 × `Content modified in rendered editor, recorded for undo` in the same second. Reproduced twice (F).
- **Reproductions:** (F) `test_github_html.md`, `…132337-R2-HTML-6.log` lines 3055–3131 (12:23:59Z = 13:23:59 CET); (G) `test_md/test_git_graphs.md` (restored tab) and `/tmp/ferrite-rerun2/G/m165.md` (copy of `test_mermaid_issue_165.md`, char count 6376 → 6374), `…133805-R2-MMD-10.log` lines 5983/6015 and 36105–36139 (`title="test_git_graphs.md* - Ferrite"`); (H) `/tmp/ferrite-rerun2/html4.md` (tester's own fixture), `…134343-R2-GIT-3.log` lines 191640–191641 and later; (H) `test_github_html.md` restored tab showed `*` (recovery file `recovery/53.json` held its content). After Don't Save + relaunch the files were clean. Not seen on `test_git_graphs.md` in Batch H.
- **Evidence:** `rerun2/R2-HTML-6-spurious-dirty.webp`, `rerun2/shots-G/R2-dirty-on-view-git-graphs.webp`; logs above.
- **Note:** may share a cause with ISS-65 (Rendered session writing back blocks it did not edit), not verified.

### ISS-69 — Linux YouTube embeds never play ("Error 153"); offline black rectangles; Gdk-CRITICAL warnings
- **Rows:** T2-36, VID-1, EXTRA-YT-2, EXTRA-YT-4 (FAIL); EXTRA-YT-6, EXTRA-YT-7 (PARTIAL). Successor to ISS-2 (the GDK abort is fixed; VID-10 PASS). Severity **major**.
- **Repro:** 1. New `.md` with `{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ}}` (or `test_md/test_video_embeds.md`). 2. **Rendered**. 3. Click the embed.
- **Expected:** thumbnail, then a playable player (or a clean degrade/placeholder).
- **Actual:** the area is first blank/white or badly clipped; clicking (or after ~6 s) shows YouTube's player error **"Feil 153" (Error 153)**, never playback. Seen on every trusted embed tried (embeds 1–4 of the 12-embed page too). Thumbnail fetch itself works (`GET https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg` → 200). Error 153 is, as far as I know, YouTube's "missing referrer / embedder identity" error, which suggests the WebView loads the player without a `Referer`/origin — **not verified**. **Offline** (`sudo unshare -n`, EXTRA-YT-2): large black WebView rectangles, no placeholder, no toast on click (Raw still works; no retry storm). **12 embeds** (EXTRA-YT-4): scrolling top→bottom×3 stays responsive (RSS 286 380 KB) but logs 20 `Gdk-CRITICAL` lines (` _gdk_frame_clock_freeze` / `_thaw: assertion 'GDK_IS_FRAME_CLOCK (clock)' failed`). In EXTRA-YT-7, the Reload confirmation and Search-in-Files dialogs did draw above the webviews (pass), webviews initially overlapped/painted black.
- **Environment caveats:** network through the sandbox proxy `198.18.0.1`; the YouTube rows ran (11:08–11:16 CET) on the WebKitGTK runtime installed before the box restart (version not recorded at the time; October's run used `2.54.0-1~deb13u2`). During the environment restore it was updated to `libwebkit2gtk-4.1-0 2.54.0-1~deb13u3`; the YouTube rows were **not** re-run on it. Please confirm on a desktop with direct network.
- **Evidence:** `logs/ferrite-debug-20261109-110845-R2-T2-36.log`, `…110956-R2-VID-1.log`, `…111035-R2-EXTRA-YT-2.log`, `…111123-R2-EXTRA-YT-4.log`, `…111221-R2-EXTRA-YT-6.log`, `…111333-R2-EXTRA-YT-6b.log`, `…111452-R2-EXTRA-YT-7.log`; `shots/R2-T2-36-fail.png`, `R2-T2-36-pass.png`, `R2-VID-1-fail.png`, `R2-EXTRA-YT-2-fail.png`, `R2-EXTRA-YT-4-fail.png`, `R2-EXTRA-YT-6-partial.png`, `R2-EXTRA-YT-7-*.png`.

### ISS-70 — Copy from a Rendered inline-formatted paragraph puts raw Markdown on the clipboard
- **Rows:** CPY-3 (ISS-1 row; the selection hang is fixed). Severity **major**.
- **Repro:** 1. Rendered, the inline-formatted paragraph ("And this inline-formatted paragraph — copy the whole line …"). 2. Select it, **Ctrl+C**. 3. `xclip -selection clipboard -o`.
- **Expected:** `And this inline-formatted paragraph — copy the whole line and verify no extra spaces appear around the styled spans: normal bold italic code end.`
- **Actual:** the source Markdown including `**bold**`, `*italic*`, `` `code` `` and a newline between source lines.
- **Evidence:** `logs/ferrite-debug-20261109-110716-R2-CPY-3.log`; `shots/R2-CPY-3-fail.png`. (Batch A's earlier CPY-3 "empty clipboard" result is void: `xclip` was not installed then.)

### ISS-71 — Tab on a Rendered list item does nothing
- **Rows:** IND-9 (ISS-1 row; hang fixed). Severity **major**.
- **Repro:** 1. Rendered, a list (`list item one` …), preview unlocked. 2. Click `list item one`. 3. Press **Tab**. 4. Switch to Raw.
- **Expected:** "Rendered list Tab (list-item indent) unchanged" — the item is indented as before.
- **Actual:** no edit, no visible caret, no indentation; Raw shows the three list lines unchanged. App responsive.
- **Evidence:** `logs/ferrite-debug-20261109-110047-IND-9.log` (clean shutdown tail, no error); screenshot not saved as a file.

### ISS-72 — Corrupt / invalid config: no toast; invalid JSON silently falls back to full defaults
- **Rows:** EXTRA-SET-9a, EXTRA-SET-9b. Successor to ISS-9. Severity **major**.
- **Repro 9a:** back up `~/.config/ferrite`; write `{ not json` to `config.json`; launch.
- **Expected (9a):** tolerant load, a **user-visible toast**, corrupt file renamed, no silent full wipe.
- **Actual (9a):** file renamed to `config.json.corrupt-<timestamp>` (good) and defaults created (`Config file not found … creating defaults with system locale`), with **no toast**: the user's settings are gone without any notice.
- **Repro 9b:** restore the good config; set `"font_size": "abc"`; launch.
- **Actual (9b):** only `font_size` reset (`WARN … Dropping invalid config field font_size … recovered with 1 invalid field(s) reset to default`), other settings kept (good), but **no toast**.
- **Evidence:** `logs/ferrite-debug-20261109-115756-R2-EXTRA-SET-9a.log`, `…115843-R2-EXTRA-SET-9b.log`; `shots/R2-EXTRA-SET-9a-fail.png`, `shots/R2-EXTRA-SET-9b-fail.png`. Config restored from backup afterwards.

### ISS-73 — Indexed `linkStyle … interpolate basis` still draws straight edges
- **Rows:** MMD-3. Successor to ISS-57. Severity **minor**.
- **Repro:** flowchart with `linkStyle 0,1,2 interpolate basis`, Rendered.
- **Expected:** curved edges, different from the same diagram without `linkStyle`. **Actual:** straight segments; no linkStyle diagnostic in the log.
- **Evidence:** `logs/ferrite-debug-20261109-115441-R2-MMD-1.log`, `logs/R2-MMD-3-log-tail.txt`; `shots/R2-MMD-3-fail.png`.

### ISS-74 — Settings toggles not persisted until graceful exit
- **Rows:** SI-4 (task 29 spot-check). Severity **major** (setting changes are lost on crash/kill).
- **Repro:** 1. Settings → Editor → untick **Show Line Numbers** (checkbox visibly unticked). 2. Wait ≥ 1 s (5 s in the first run). 3. `pkill -9 ferrite`. 4. Inspect `~/.config/ferrite/config.json`.
- **Expected:** saved within ~500 ms / on dialog close (task 29). **Actual:** `config.json` mtime and md5 unchanged, still `"show_line_numbers": true`. Second attempt (1 s) same. Batch G also saw the spell-check setting persist only after Alt+F4.
- **Evidence:** `logs/ferrite-debug-20261109-134250-R2-SI-4.log` (ends `exit code 137 SIGKILL`, no panic), `…134308-R2-SI-4b.log`; `shots/R2-SI-4-fail.webp`.

### ISS-75 — Spell hint tooltip drawn over the open context menu
- **Rows:** SPL-3 (task 31 spot-check). Severity **polish**.
- **Repro:** spell check on; hover a misspelled word (`speling`) until `[hint/spell] Unknown word: speling` appears; right-click it.
- **Expected:** tooltip hidden while the context menu is open. **Actual:** tooltip stays on top and covers the first suggestion row. No log error.
- **Evidence:** `logs/ferrite-debug-20261109-134343-R2-GIT-3.log` (primary instance), `…134553-R2-SPL-3.log` (forwarded launch); `shots/R2-SPL-3-fail.webp`.

## 6. Notable PARTIALs and observations needing follow-up (not filed as issues)

| Row / item | What remains | Suggested follow-up | Status (round-3 triage) |
|---|---|---|---|
| UX-9 (ISS-42) | Ctrl+click on a local file link opens the parent folder (Chrome `file://` directory index, since the box has no file manager), not the file. Batch A saw the same for a `.md` target. Plain click still edits. | Decide whether "reveal parent" is the intended Linux behaviour for `.md` targets (the re-run doc says "opens target"); check on a desktop with a file manager. | **by design** — reveal-in-file-manager without a file manager → directory index (PRD §8) |
| T3-44 (ISS-33) | Adjacent single-line `<div align="center">` / `<div align="right">` show their own text but both render left-aligned. | Apply `align` to the single-line form (October's expected result was centred / flush right). | **deferred 0.3.2** — PRD task 4 |
| EXTRA-YT-6 / EXTRA-YT-7 (ISS-2) | No crash; live URL edit/width update and Settings-over-player stacking not verifiable (xdotool appended instead of replacing; Settings opens as a tab). | Re-check after ISS-69 on a real keyboard. | re-run after PRD task 1 (round-3 rerun §D) |
| MMD-1 (ISS-8) | Popup Hand mode does not pan while the diagram fits the viewport. | Minor; decide if panning at fit size is expected. | **by design** (PRD §8) |
| EXTRA-SET-7 (ISS-56) | ja/zh-Hans Keyboard group headers and 4 command rows still English; zh-Hans accent/colour/view-mode strings and exit-dialog body English. | Add the missing locale keys. Screenshots: `shots/R2-EXTRA-SET-7-ja-keyboard.webp`, `-zh-keyboard.webp`. | **deferred 0.3.2** — PRD task 4 |
| TTL-6a (ISS-54) | Double-click maximize/restore works; drag-from-maximized not testable with synthetic drag. | Check on a real display. | re-test on real display (round-3 rerun §F) |
| T1-8 | Closing the other tab (× or middle-click) while the Reload dialog is open dismisses the dialog; re-triggering works; other file untouched. | Minor UX; probably fine. | **by design** (PRD §8) |
| SES-6 (ISS-36) | One Ctrl+Z after typing ` W1` removed `W1` but left the leading space (word-granular undo); quick-test A3 says it should remove ` W1`. Not data loss. | Minor. | **by design** — word-granular coalescing (round-1 ISS-6 rule); quick-test A3 wording corrected |
| SES-3 window (ISS-63) | **Observation, not filed:** a second window holding only a dirty Untitled quick note did not close on its × or Alt+F4 and showed no prompt (Batch E, 13:06–13:12 CET, seen once, not retested). With a named dirty file the close dialog works (Batch H). Evidence: `logs/ferrite-debug-20261109-130614-R2-SES-5.log`, `shots/R2-SES-3-no-dialog.png`. | Re-test: does an Untitled-only secondary window refuse to close? | **open — PRD task 3** (investigate `handle_window_close_request → request_close_window` for Untitled-only windows) |
| SI-1 (ISS-34) | A **minimized** primary receives the forwarded tab but is not restored/raised (`_NET_WM_STATE_HIDDEN` stays). Covered-window case raises correctly. | Check whether xfwm4 or the app is responsible. | re-test with `xprop` (round-3 rerun §F); WM-side until shown otherwise |
| SI-3 (task 28) | "Does not clear primary crash recovery" half not exercised (recovery dir empty). | Re-check with a dirty primary. | re-test (round-3 rerun §F) |
| MMD-10 (ISS-60) | Right edge of the `alt` frame clipped inline; in the `autonumber off` variant the alt label overlaps a message line. | Cosmetic. | **deferred 0.3.2** — PRD task 4 |
| HTML-4 (task 32) | `<kbd>` inside headings renders as plain text (no chip). | Cosmetic. | **deferred 0.3.2** — PRD task 4 |
| T1-4b | Editor text of the UTF-16 file renders faint grey. | Cosmetic, not investigated. | **deferred 0.3.2** — PRD task 4 |
| T1-13 | Status-bar line count stale during duplicate/move-line sequence. | Cosmetic (repaint lag, see §7). | **deferred 0.3.2** — PRD task 4 |
| CPY-2 / T1-4…T1-16 | First attempts failed for harness reasons (empty clipboard before the selection registered; no keyboard focus). | None; noted for transparency. | closed (harness) |
| Fixture typo | `test_md/test_github_html.md` line 1 is `A# GitHub HTML Subset Test` (since `570264b`). | Fix the fixture so its title renders as a heading. | **fixed (working tree)** |
| `cargo test` count | 2228 on Linux vs 2239 on Windows (11 fewer, same delta as October). | Confirm the 11 are `cfg(windows)` tests. | **confirmed** — 7 `path_utils::tests::windows_tests`, 2 `path_utils::tests`, 2 `vcs::git::tests` (names in round-3 rerun §F) |

## 7. Environment notes and harness limits

- **Box restart mid-Batch C.** The box restarted at about 11:36 CET, after T1-4b/RLD-7/T1-15/SIF-1–3 and a first T1-19 attempt. The release build and the screenshot/clipboard tools (`scrot`, `xclip`) were lost. The environment was restored (`rerun2/env-restore.md`): same HEAD `9effd76`, clean tree, `cargo build --release --locked` PASS (3 m 33 s, binary 46 626 640 bytes, mtime 11:48 CET, log `logs/rerun2-rebuild.log`); `pkg-config`, `make` (needed for jemalloc), `scrot 1.12.1-1`, `xclip 0.13-4`, `gdb 16.3-1`, ImageMagick 7.1.1-43 reinstalled via `curl` + `dpkg -i` (apt over the proxy still returns 500); smoke launch OK (`shots/R2-SMOKE-pass.png`). `cargo test` was not re-run after the rebuild. All T1-19/UND-1 results are from the fresh build. Stale `instance.{lock,pid,port}` files were cleared after the smoke SIGTERM.
- **WebKitGTK version change.** During the restore the runtime `libwebkit2gtk-4.1-0` was updated to **`2.54.0-1~deb13u3`** (security update). Batch B (all YouTube/video rows, 11:07–11:16 CET) ran **before** that on the previously installed runtime. `libwebkit2gtk-4.1-dev` remains half-configured (version skew), which does not affect the runtime.
- **Display:** Xvfb `:2` 1920×1200, xfwm4, no compositor, Mesa llvmpipe. Ferrite repaints lazily under Xvfb/llvmpipe: screenshots can lag one action behind until the pointer moves; testers jiggled the mouse and used the status bar and files on disk as the source of truth.
- **xfwm4 grabs** Alt+click (easy_click = Alt) and Alt+Space (window menu); the WM setting was not changed. Alt+click multi-cursor and the Alt+Space command palette are unreachable here; T1-20 used Ctrl+click as the checklist says. Alt+F4 works. Real Alt+Z reached the app (T2-39 PASS).
- **xdotool chord timing:** atomic Ctrl-chords (Ctrl+S, Ctrl+Shift+N, Ctrl+Shift+R) were sometimes ignored; holding the modifier ~0.4 s (`keydown ctrl; key s; keyup ctrl`) worked. Batch D's first-pass T1-4/5/6/8/16 FAILs were this plus missing text focus, and were re-run as PASS. Fast synthetic typing also dropped a CJK character (T1-21 in Batch B) and appended instead of replacing a selection (EXTRA-YT-6).
- **Synthetic drag:** mouse drags (title-bar drag-to-restore TTL-6a, Mermaid Hand-pan at some zoom levels) are unreliable; affected steps are PARTIAL, not FAIL.
- **No file manager** on the box: "reveal in file manager" falls back to Chrome showing a `file://` directory index (UX-9). The native file dialog (xdg-desktop-portal) does not work, so files were opened through CLI/single-instance forwarding.
- **Clipboard:** image clipboard owner was `xclip` (only `image/png` target); no screenshot tool. arboard also logs `Could not hand the clipboard contents over to the clipboard manager` on exit (no clipboard manager running).
- **Network / video:** network goes through the sandbox proxy `198.18.0.1`; YouTube thumbnails download, but playback never worked (Error 153, ISS-69), so playback-dependent steps (T2-37 "play video in window 1", EXTRA-YT-7 overlay-on-playing-video) could not be exercised. Offline test used `sudo unshare -n`.
- **Shared config:** `~/.config/ferrite` was backed up before Batches C, E, F, G, H (`rerun2/config-backup*/`) and restored after each (`diff -r` clean in H). A stale `mistaek` entry in `user-words.txt` was cleared for SPL-2.
- **Batch A screenshots:** Batch A ran before `scrot` was available; its captures were made with the computer-use tool and referenced as agent asset URLs that are not readable on the box, so they could not be put in the zip. Batch A logs are included.
- **End state:** no Ferrite process left after each batch; `git -C ferrite status --short` empty; nothing committed, pushed or filed.

## 8. Attachments

| File | Contents |
|---|---|
| `rerun2/rerun2-shots.zip` (13,469,222 bytes, 86 files) | All re-run screenshots: `shots/R2-*` (incl. `.webp`), `shots/RERUN-D2-*` (focus-confirmed Tier 1 re-runs), `rerun2/R2-HTML-6-spurious-dirty.webp`, `rerun2/R2-UX-9-chrome-dir-index.webp`, `rerun2/T1-2-startup.webp`, `rerun2/shots-G/R2-*.webp` |
| `rerun2/rerun2-logs.zip` (9,990,024 bytes, 133 files) | **all** 88 `logs/ferrite-debug-20261109-*.log` files of this re-run (every FAIL/PARTIAL row's log plus the PASS rows' logs), `logs/R2-*.txt` and `logs/RERUN-D2-*.txt` check/tail files, `rerun2/R2-UND-3-diff.txt`, `rerun2/R2-UND-3-corrupted-file.md`, `logs/rerun2-build.log`, `logs/rerun2-cargo-test.log`, `logs/rerun2-rebuild.log`, `logs/rerun2-clippy.log`, and the raw batch results `rerun2/results-*.md`, `automated-gate.md`, `env-restore.md`, `mmd10-sequence-variants.md` |
