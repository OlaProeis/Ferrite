# Ferrite v0.3.1 QA re-run report (round 4) — Linux

- **Commit tested:** fix `e56243c` (`fix(qa): v0.3.1 GrokBot round 4 direct fixes (ISS-76/77/78/79, SES-3)`), branch `0.3.1-experimental`, HEAD `1ad150b` (`docs(qa): GrokBot round 3 report, round 4 re-run gate for e56243c (ISS-76..79, SES-3)`). Working tree was clean; fast-forwarded `d1d4d15` → `1ad150b` (2 commits). Binary `target/release/ferrite`, `ferrite 0.3.1`, 46 640 680 bytes, mtime **00:00 CET**.
- **Gate doc followed:** `grokbot/november/rerun-v0.3.1-round4.md` (copy at `rerun4/rerun-v0.3.1-round4.md`). Prior baseline: `grokbot/november/rerun3/qa-rerun3-report-v0.3.1-linux.md` (fix `8c9c2d5` / HEAD `d1d4d15`).
- **Date / timezone:** Tuesday 2026-11-10, GUI testing ~00:02–00:47 CET (Europe/Oslo, UTC+1). Times in this report are CET unless they end in `Z`; debug-log timestamps are UTC (CET = Z + 1 h).
- **Environment:** Debian GNU/Linux 13 (trixie). **X11 only:** Xvfb display `:2`, 1920×1200×24, xfwm4, no compositor. **No GPU:** Mesa **llvmpipe**. WebView stack WebKitGTK 4.1 via wry — `libwebkit2gtk-4.1-0` **2.54.0-1~deb13u3**.
- **File dialogs (SES-3d):** The box initially had **no** `xdg-desktop-portal` / zenity, so rfd could not open Save As (`pick_folder error No such file or directory` → `Save dialog cancelled`). Mid-round the following were installed from Debian pool debs (`logs/portal-setup.log`): **xdg-desktop-portal 1.20.3+ds-1**, **xdg-desktop-portal-gtk 1.15.3-1**, **zenity 4.1.90-1**, plus fuse3 / libfuse3. SES-3d was then re-run via `rerun4/run-ferrite-portal.sh` (DISPLAY=:2, session bus, XFCE desktop, FileChooser=gtk) and **PASS**ed.
- **Toolchain:** `rustc` / `cargo` as on the box for this build.
- **Build time (release):** 2 m 12 s (`cargo build --release --locked`, exit 0). Evidence: `rerun4/logs/build.log`.
- **Tester:** Grok Bot, driving the GUI with synthetic X11 input (xdotool / computer-use) through `run-ferrite.sh` (`--log-level debug`, exit code recorded); SES-3d portal re-run used `run-ferrite-portal.sh`. **Tester only:** no Ferrite code was changed, nothing was committed, pushed, or filed as a PR/issue.
- **Raw results:** `rerun4/prep.md`, `results-A.md`, `results-B.md`, `results-C.md` (incl. `## SES-3d re-run with portal`), `results-D.md` (also inside `rerun4-logs.zip` / `rerun4-files.zip` as listed in §8).
- **Attachments:** `rerun4/rerun4-shots.zip` (downscaled JPEG copies of screenshots; originals remain under `rerun4/shots/`), `rerun4/rerun4-logs.zip` (debug logs + batch results + portal-setup), `rerun4/rerun4-files.zip` (saved files, diffs, results-*.md, prep.md). Paths in the Evidence columns below are relative to `rerun4/` (or the zip root: `shots/…`, `logs/…`, `files/…`, `work/…`).

## 0. Verdict

**Round-4 gate criteria met for every runnable row.** Every row is **PASS** or **BLOCKED-by-build / decision / WM / environment**. **0 FAIL.** No ISS-80+ filed. Tier 1 rows in scope (T1-17, T1-18, T1-19) all **PASS**.

**Tagging `v0.3.1` still depends on ISS-69 (YouTube / task 1),** which remains **BLOCKED-by-build** (no task-1 commit in `git log` since `d1d4d15`). Until those rows run and pass (or are explicitly waived), do not treat this report as a green light to tag.

**Blocking / waiting (not new FAILs):**

| Item | State | Summary |
|---|---|---|
| ISS-69 / T2-36, VID-1, EXTRA-YT-2/4/6/7 | **BLOCKED-by-build** | Task-1 commit absent; Section E not run |
| T3-44 | **PARTIAL** (known, 0.3.2 / task 4) | Task-4 commit absent; not re-run |
| CPY-3, IND-9 | **BLOCKED-by-decision** | PRD §8; not repeated |
| SI-1, TTL-6a | **BLOCKED-by-WM / environment** | Xvfb limits; not repeated |

**Tier 1 rows re-checked this round (the ones the gate says the fixes touch):**

| Tier 1 row | Round 3 | Round 4 final | Notes |
|---|---|---|---|
| T1-17 | FAIL (ISS-78) | **PASS** | 15 `Rendered arrow nav Down` lines incl. `34 → 44`; `X` on line 49 |
| T1-18 | PASS | **PASS** (sanity) | `x` inside blockquote (`> xA quoted…`) |
| T1-19 | PASS | **PASS** (sanity) | Only line 42 gains `x`; one Ctrl+Z byte-identical |

Good news: **no hang and no crash in any row of this re-run**; every Ferrite process used for a graded row exited with code 0 (one Mermaid session was SIGTERM'd at cleanup = exit 143, not a crash). Config was restored byte-identical (`cmp`) after every batch that touched it. Direct fixes for ISS-76, 77, 78, 79 and SES-3 are confirmed on this build.

**Counts (33 rows, final results, §4):** **22 PASS / 0 FAIL / 1 PARTIAL / 10 BLOCKED** (0 SKIPPED).

| Set | Rows | PASS | FAIL | PARTIAL | BLOCKED |
|---|---:|---:|---:|---:|---:|
| A. Rendered arrow nav (ISS-76/77/78) | 7 | 7 | 0 | 0 | 0 |
| B. Mermaid curve (ISS-79) | 2 | 2 | 0 | 0 | 0 |
| C. Secondary window close (SES-3) | 8 | 8 | 0 | 0 | 0 |
| D. Sanity + cargo test | 5 | 5 | 0 | 0 | 0 |
| E. Still waiting / not repeated | 11 | 0 | 0 | 1 | 10 |
| **Total** | **33** | **22** | **0** | **1** | **10** |

BLOCKED breakdown: **6 by-build** (Section E YouTube ×6), **2 by-decision** (CPY-3, IND-9), **1 by-WM** (SI-1), **1 by-environment** (TTL-6a / Xvfb). T3-44 counted as PARTIAL (task 4 / 0.3.2), not FAIL.

## 1. Automated gate

| Check | Result | Numbers |
|---|---|---|
| Correct commit / clean tree | PASS | HEAD `1ad150b`, fix ancestor `e56243c` present; tree clean before fast-forward. Task 1 and task 4 cyclopsctl commits **absent** (see prep.md). |
| Build markers | PASS | `Rendered arrow nav` ×2; `take_locked_preview_arrow_presses` ×3; `basis_curve_segments` ×6; `close_document_window_now` ×2 (all ≥ 1). |
| `cargo build --release --locked` | PASS | Finished in 2 m 12 s, exit 0. Binary 46 640 680 bytes. Evidence: `logs/build.log` |
| `cargo test --locked` | PASS | **2249 passed / 0 failed / 2 ignored** (exit 0). `running 2251 tests` (= 2249 + 2 ignored). Matches gate §D (round 3: 2235 + 14 new = 2249). Flaky `app::rendered_commit_point_tests::switch_tab_commits_previous_tab_content` **passed** in the full run (no solo re-run needed). Ignored: `editor::ferrite::buffer::tests::test_large_file`, `editor::ferrite::history::tests::test_large_buffer_performance`. Evidence: `logs/cargo-test.log` |
| Tools before first GUI row | PASS | `scrot` 1.12.1, `xclip` 0.13, `maim` 5.8.0, `xdotool`, ImageMagick; later also portal + zenity (see §7) |

## 2. Verdict against the doc's pass criteria

| Criterion (`rerun-v0.3.1-round4.md`) | Met? | Detail |
|---|---|---|
| 1. Every row is PASS or BLOCKED-by-input/environment/build/decision | **Yes** | 0 FAIL. 22 PASS; 10 BLOCKED (build / decision / WM / env); 1 known PARTIAL (T3-44 / task 4 — not a new FAIL, same status as round 3). |
| 2. Tier 1 of the minimal checklist: all PASS or BLOCKED-by-input/environment | **Yes** | T1-17 **PASS**; sanity re-checks T1-18 and T1-19 **PASS**. |
| 3. Screenshots + debug logs for FAILs; saved files where required | N/A (no FAIL) | Evidence still attached for every PASS under `shots/`, `logs/`, `files/`. |
| 4. Rows that passed in round 3 not repeated except Section D sanity | Yes | Only A/B/C (fix targets) + D sanity + cargo test were run; E left blocked. |

Against the checklist "Tag gate (minimal)": the round-4 gate's own criteria are met, **but tagging still waits on ISS-69 (YouTube / task 1)**. Suggested next step: land task 1, then re-run Section E YouTube rows (and task 4 / T3-44 if still in scope for 0.3.1).

## 3. Per-ISS summary (ISS-76 … ISS-79 from round 3, plus SES-3)

Status key: **Fixed** = every re-checked row for that ISS is PASS. **Blocked** = not runnable on this build (task commit missing). **Deferred / by-design** = gate says BLOCKED-by-decision.

| ISS / item | Round-4 status | Rows → final | Notes |
|---|---|---|---|
| ISS-76 | **Fixed** | T2-35 PASS; T2-35s PASS; T2-35u PASS | Locked preview: no caret, Up/Down scroll (`Applied nav button scroll offset…` 42-step), no `Rendered arrow nav`, `Q` not inserted. Unlock restores block nav. |
| ISS-77 | **Fixed** | ARR-2 PASS; ARR-2s PASS | Status bar follows Rendered caret (`Ln 8`→`10`, `Ln 50`→`51`, Col +1 on type). Split still shows Raw Ln (unchanged). ARR-2(a) read `Col 5` not `Col 4` because the click landed one character into the heading text — not a FAIL. |
| ISS-78 | **Fixed** | T1-17 PASS; T1-17b PASS | 15 nav lines incl. fence step `34 → 44`; `X` on line 49. From line 10, 15 presses → `X` on line 34; 16th press → line 44. |
| ISS-79 | **Fixed** | MMD-3 PASS; MMD-3b PASS | With `linkStyle … interpolate basis`: four S-curves; without: straight diagonals. Straight chain `A→B→C→D` stays straight with or without linkStyle. |
| SES-3 | **Fixed** | SES-3a…3h all PASS | Window-close prompts like tab-close; Alt+F4 on clean secondary closes; Quick-note ON/OFF same prompt text; system title bar OK; app-exit restore of `abc` OK. SES-3d needed portal install (env) then PASS. |
| ISS-69 | **Blocked** (task 1) | T2-36, VID-1, EXTRA-YT-2/4/6/7 → BLOCKED-by-build | Not run. |
| ISS-70 / ISS-71 | **Deferred / by-design** | CPY-3, IND-9 → BLOCKED-by-decision | Not repeated. |
| T3-44 | **PARTIAL** (task 4 / 0.3.2) | BLOCKED-by-build / known PARTIAL | Not re-run. |

## 4. Full row table (final results)

Supersession / grading notes:

- **T1-17 and ARR-2:** first attempts voided by harness typos (accidental extra `x`; missed click + double-typed `z`/`Z`). Voided logs kept (`logs/A-ARR-2-attempt1-doubletype.log`; void T1-17 log outside `rerun4/logs/`). Clean re-runs are authoritative → **PASS**.
- **SES-3d:** first graded runs (results-C initial + results-B confirm ON/OFF + tab-close control) could not open Save As because the box had no xdg-desktop-portal. Those runs are **environment evidence only**, not a product FAIL. After installing the portal stack and re-running via `run-ferrite-portal.sh` → final **PASS** (results-C `## SES-3d re-run with portal`).
- **Earlier observation** that a cancelled Save As left the window-close dialog with "no feedback" was an **artefact of the missing portal** (rfd returned cancelled immediately). With the portal, Cancel returns to the still-visible Unsaved Changes close prompt — correct behaviour; not filed.

### 4.1 Section A — Rendered arrow navigation (ISS-76, 77, 78)

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| T1-17 | ISS-78 | **PASS** | A | shots/A-T1-17-*.png; files/A-T1-17*; logs/A-T1-17.log | Unlocked; 15 nav lines targets `10…49` incl. `34→44`; `X` at start of line 49 |
| T1-17b | ISS-78 | **PASS** | A | shots/A-T1-17b-*.png; files/A-T1-17b*; logs/A-T1-17b.log | 15 presses → `X` on line 34; 12th nav line `34→44` |
| ARR-2 | ISS-77 | **PASS** | A | shots/A-ARR-2-{a,b,c1,c2,d}-status.png, -padlock.png; logs/A-ARR-2.log | (a) `Ln 8, Col 5` (click offset into heading; never `Ln 1, Col 1`); (b) `Ln 10, Col 1`; (c) `Ln 50`→`51`; (d) Col +1 |
| ARR-2s | ISS-77 | **PASS** | A | shots/A-ARR-2s-*.png; logs/A-ARR-2s.log | Split: status bar stays Raw `Ln 1, Col 1` while preview caret moves |
| T2-35 | ISS-76 | **PASS** | A | shots/A-T2-35-*.png; files/A-T2-35.*; logs/A-T2-35.log | Locked: scroll offsets 42…210 then back to 0; `Q` not inserted; diff empty; zero nav lines |
| T2-35s | ISS-76 | **PASS** | A | shots/A-T2-35s-*.png; logs/A-T2-35s.log | Preview scrolls while locked; Raw still editable (`R` then Ctrl+Z) |
| T2-35u | ISS-76 | **PASS** | A | shots/A-T2-35u-*.png; logs/A-T2-35u.log | Unlock → caret + block nav back (`Some(12)→16`, `Some(16)→18`) |

Config restored from `config-backup-start/` after this batch (`cmp`).

### 4.2 Section B — Mermaid curve (ISS-79)

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| MMD-3 | ISS-79 | **PASS** | B | shots/B-MMD3-with-linkStyle-200.png, -without-linkStyle-200.png, -edgeAB-sidebyside.png, -diff.png | Four S-curves with linkStyle; straight diagonals without; whole-edge difference |
| MMD-3b | ISS-79 | **PASS** | B | shots/B-MMD3b-with-linkStyle-200.png, -without-linkStyle-200.png, -diagram-stacked-with-over-without.png | Straight chain identical with/without linkStyle (only 24 px border noise) |

Config restored from `config-backup-B/` after this batch (`cmp`).

### 4.3 Section C — Secondary window close (SES-3)

| Row | Final | Batch | Evidence | Notes |
|---|---|---|---|---|
| SES-3a | **PASS** | C | shots/C-3a-*.png; logs/C-3a.log | Clean secondary: Alt+F4 closes at once; `Secondary window 1 closed` / `Closed document window 1` |
| SES-3b | **PASS** | C | shots/C-3b-dialog.png, -dialog-crop.png, -cancelled3.png; logs/C-3b.log | Body `Unsaved changes in 'Untitled'. Close this window anyway?`; no prohibited phrase; Cancel keeps `abc` |
| SES-3c | **PASS** | C | shots/C-3c-*.png; logs/C-3c.log | Alt+F4 same dialog; Don't Save closes secondary |
| SES-3d | **PASS** | C (portal re-run) | shots/C-3d-portal-*.png; files/SES-3d-abc.md (3 bytes `abc`); logs/C-3d-portal.log | GTK Save File chooser; saved; window closed; Cancel of Save As returns to close prompt. Pre-portal runs: env evidence only (see §7) |
| SES-3e | **PASS** | C | shots/C-3e-*.png; logs/C-3e.log | Quick-note OFF: same dialog text; Cancel / Don't Save OK |
| SES-3f | **PASS** | C | shots/C-3f-*.png; logs/C-3f.log | Dirty primary with second window open: same dialog; Cancel keeps both |
| SES-3g | **PASS** | C | shots/C-3g-*.png; logs/C-3g.log, C-3g-relaunch.log | Last-window exit: no dialog; relaunch restores Untitled with `abc` |
| SES-3h | **PASS** | C | shots/C-3h-*.png; logs/C-3h.log | System title bar: native × matches 3a/3b; option restored OFF |

Config restored from `config-backup-C/` after the main C batch and from `config-backup-3d/` after the portal re-run (`cmp` both).

#### SES-3d environment note (pre-portal)

Before the portal install, Save from the window-close dialog (and from a plain tab-close control) logged:

```text
rfd::backend::xdg_desktop_portal ERROR pick_folder error No such file or directory (os error 2)
ferrite::files::dialogs Save dialog returned None on unknown (portal-requiring desktop)…
ferrite::app::file_ops Save dialog cancelled
```

Evidence of those attempts: `shots/C-3d-*.png`, `C-3d-confirm-ON-*.png`, `C-3d-confirm-OFF-*.png`, `C-3d-control-after.png`; `logs/C-3d.log`, `ferrite-debug-*-R4-3d-ON.log`. Graded **BLOCKED-by-environment** at the time; final grade after portal re-run is **PASS**.

### 4.4 Section D — Sanity rows + cargo test

| Row | Final | Batch | Evidence | Notes |
|---|---|---|---|---|
| T1-18 | **PASS** | D | shots/D-T1-18-*.png; files/D-T1-18.*; logs/D-T1-18.log | Unlocked; `Some(29)→32`; `> xA quoted…`; line 29 unchanged |
| ARR-4 | **PASS** | D | shots/D-ARR-4-*.png; files/D-ARR-4-arrow-nav-grep.txt; logs/D-ARR-4.log | Preview walks blocks; Raw `Ln 1, Col 1` unchanged |
| T1-19 | **PASS** | D | shots/D-T1-19-*.png; files/D-T1-19.*; logs/D-T1-19.log | Only line 42 gains `x`; one Ctrl+Z → empty diff / cmp identical |
| ISS-68 | **PASS** | D | shots/D-ISS-68-*.png; logs/D-ISS-68.log | No `*`; `Content modified in rendered editor` count = 0 |
| `cargo test` count | **PASS** | prep | logs/cargo-test.log; prep.md | **2249 / 0 / 2**; flaky switch-tab test passed |

Config restored from `config-backup-D/` after this batch (`cmp`).

### 4.5 Section E — Still waiting / not repeated

| Row | Final | Notes |
|---|---|---|
| T2-36 | **BLOCKED-by-build** | ISS-69 / task 1 not in `git log` |
| VID-1 | **BLOCKED-by-build** | Same |
| EXTRA-YT-2 | **BLOCKED-by-build** | Same |
| EXTRA-YT-4 | **BLOCKED-by-build** | Same |
| EXTRA-YT-6 | **BLOCKED-by-build** | Same |
| EXTRA-YT-7 | **BLOCKED-by-build** | Same |
| T3-44 | **PARTIAL** (known / task 4 / 0.3.2) | Task-4 commit absent; not re-run |
| CPY-3 | **BLOCKED-by-decision** | ISS-70; PRD §8; not repeated |
| IND-9 | **BLOCKED-by-decision** | ISS-71; deferred 0.3.2; not repeated |
| SI-1 | **BLOCKED-by-WM** | Not repeated under Xvfb |
| TTL-6a | **BLOCKED-by-environment** | Not repeated under Xvfb |

`libwebkit2gtk-4.1-0` = **2.54.0-1~deb13u3** (recorded for when task 1 lands).

## 5. New issues (ISS-80 …)

**None.** Highest prior ID is ISS-79. No FAIL rows this round, so nothing was filed as ISS-80+ and the round-4 gate / PRD Expected columns were not edited.

## 6. Notable observations (not filed as issues)

| Item | What | Suggested follow-up |
|---|---|---|
| Return while close dialog open | With the window-close Unsaved Changes dialog open, pressing **Return** inserted a newline into the document (dialog is not modal for keyboard / Return does not activate a button). Seen during SES-3d troubleshooting (results-C). Note discarded; no file persisted. | Product decision: make the dialog consume Return (e.g. default button) or otherwise block editor key input while it is up. |
| Mermaid "Source >" at 200 % | Expanding the diagram header Source toggle in Rendered at 200 % zoom showed truncated source (only `flowchart LR` / `A --> B`). Not used for editing; not graded against any row (results-B). | Optional UI polish; not a gate fail. |
| Ctrl+Q = Blockquote | In Rendered/editor, **Ctrl+Q** is the Blockquote shortcut (log: `Ctrl+Q (Blockquote)`), not quit. Used twice by harness mistake; Don't Save; disk unchanged. Quit with Alt+F4 afterwards. | Harness note for future rounds. |
| ARR-2 Col 4 vs Col 5 | Gate expects `Col 4` at the start of `## Section 1` text; this run read `Col 5` because the click landed one character into the heading. Ln tracking itself is correct. | Gate / harness: aim click at heading text start, or accept Col ≥ 4 as long as Ln is right and never `Ln 1, Col 1`. |
| Cancelled Save As (pre-portal) | Window-close dialog stayed up with no chooser when portal was missing. With portal present, Cancel returns to the close prompt (correct). | No action; earlier "no feedback" observation withdrawn as env artefact. |
| Tab bar wrap | With 45+ tabs the tab bar wraps to 4 rows; click coordinates differ from early screenshots. | Harness only. |

## 7. Environment notes and harness limits

- **Display:** Xvfb `:2` 1920×1200, xfwm4 (grabs Alt+click / Alt+Space), Mesa llvmpipe. Maximised Ferrite window used so the status bar is visible.
- **WebKitGTK:** `libwebkit2gtk-4.1-0` **2.54.0-1~deb13u3** (YouTube Section E not run on it this round).
- **xdg-desktop-portal (added mid-round):** `xdg-desktop-portal` **1.20.3+ds-1**, `xdg-desktop-portal-gtk` **1.15.3-1**, `zenity` **4.1.90-1** (plus fuse3). Installed via curl + dpkg from deb.debian.org because apt is still broken behind the sandbox proxy. Setup log: `logs/portal-setup.log`. Launcher for SES-3d: `run-ferrite-portal.sh` (session bus under `/tmp/xdg-runtime-box-2`, `XDG_CURRENT_DESKTOP=XFCE`, FileChooser=gtk). `gdbus FileChooser.version` → 4.
- **xdotool:** Ctrl chords held ~0.4 s; ArrowDown presses ≥ 100–200 ms apart.
- **Config:** backed up under `rerun4/config-backup-{start,C,B,D,3d}/`; restored with `cmp` after every batch. No Ferrite process left running after each batch.
- **Padlock:** confirmed per row (crops in `shots/`); fresh `work/` copies of fixtures used so session lock state from round 3 paths was not reused.
- **End state:** no source changes; nothing committed, pushed, or filed. Original full-resolution PNGs remain in `rerun4/shots/`; the shots zip carries downscaled JPEGs (max width ~1280, q=80) to stay under attachment size limits.

## 8. Attachments

| File | Size | Contents |
|---|---|---|
| `rerun4/qa-rerun4-report-v0.3.1-linux.md` | (this file) | Round 4 Linux QA report |
| `rerun4/rerun4-shots.zip` | **7.70 MB**, 150 files | Downscaled JPEG copies of `shots/*` (max width 1280, q=80). Originals: `rerun4/shots/` (150 files) |
| `rerun4/rerun4-logs.zip` | **6.33 MB**, 30 entries | All `logs/*` from this re-run (incl. `portal-setup.log`, `build.log`, `cargo-test.log`) |
| `rerun4/rerun4-files.zip` | **21 KB**, 21 entries | `files/*` (saved markdown, diffs, greps) + `results-A.md` … `results-D.md` + `prep.md` |
