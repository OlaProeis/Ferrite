# Ferrite v0.3.1 QA re-run report (round 3 close-out) — Linux

- **Commit tested:** fix `8c9c2d5` (`fix(qa): v0.3.1 GrokBot round 3 direct fixes (ISS-64/65/67/68/72/74/75)`), branch `0.3.1-experimental`, HEAD `d1d4d15` (`docs(qa): reference round-3 fix commit 8c9c2d5 in PRD, report and re-run gate`). Working tree clean at start (`git status --short` empty). Binary `target/release/ferrite`, `ferrite 0.3.1`, 46 632 848 bytes, mtime **15:34 CET**.
- **Gate doc followed:** `grokbot/november/rerun-v0.3.1-round3.md` (copy at `rerun3/rerun-v0.3.1-round3.md`). Prior baseline: `grokbot/november/qa-rerun2-report-v0.3.1-linux.md` (code `221b6ae` / HEAD `9effd76`).
- **Date / timezone:** Monday 2026-11-09, GUI testing ~15:45–16:55 CET (Europe/Oslo, UTC+1). Times in this report are CET unless they end in `Z`; debug-log timestamps are UTC (CET = Z + 1 h).
- **Environment:** Debian GNU/Linux 13 (trixie). **X11 only:** Xvfb display `:2`, 1920×1200×24, xfwm4, no compositor. **No GPU:** Mesa **llvmpipe**. WebView stack WebKitGTK 4.1 via wry — `libwebkit2gtk-4.1-0` **2.54.0-1~deb13u3**.
- **Toolchain:** `rustc` / `cargo` as on the box for this build.
- **Build time (release):** 2 m 42 s (`cargo build --release --locked`, exit 0). Evidence: `rerun3/build-release.log`.
- **Tester:** Grok Bot, driving the GUI with synthetic X11 input (xdotool / computer-use) through `run-ferrite.sh` (`--log-level debug`, exit code recorded). **Tester only:** no Ferrite code was changed, nothing was committed, pushed, or filed as a PR/issue.
- **Raw results:** `rerun3/prep.md`, `results-A1.md`, `results-A2.md`, `results-B.md`, `results-C.md`, `results-EF.md` (also inside `rerun3-logs.zip`).
- **Attachments:** `rerun3/rerun3-shots.zip` (downscaled JPEG copies of screenshots; originals remain under `rerun3/shots/`), `rerun3/rerun3-logs.zip` (debug logs + batch results), `rerun3/rerun3-files.zip` (saved files, diffs, pasted assets). Paths in the Evidence columns below are relative to `rerun3/` (or the zip root: `shots/…`, `logs/…`, `files/…`, `work/…`).

## 0. Verdict

**NOT ready to tag.** The pass criteria in `rerun-v0.3.1-round3.md` are not met: **4 rows FAIL** (filed below as **ISS-76 … ISS-79**; the round-3 PRD was not edited) and Tier 1 still has **1 FAIL** (T1-17 → ISS-78).

**Blocking Tier 1 FAIL:**

| Tier 1 row | New issue | Summary |
|---|---|---|
| T1-17 | ISS-78 | In Rendered-only (preview unlocked), ArrowDown walks visual rows down to the paragraph just above the Rust code block (`The word ferrite appears here again.`) and cannot get past it; presses beyond that do nothing. Split *can* pass the same code block (ARR-4 probe). |

**Tier 1 rows re-checked this round (the ones the gate says the fixes touch):**

| Tier 1 row | Round 2 | Round 3 final | Notes |
|---|---|---|---|
| T1-14 | FAIL (ISS-66) | **PASS** | Image paste works on X11 (xclip and maim owners) |
| T1-17 | FAIL (ISS-64) | **FAIL** (ISS-78) | Rendered-only caret stuck above fenced code block |
| T1-18 | FAIL (ISS-64) | **PASS** | With padlock unlocked, `x` lands inside the blockquote |
| T1-19 | FAIL (ISS-65) | **PASS** | One-line edit + undo/redo byte-identical; no stray heading |
| T1-20 | FAIL (ISS-67) | **PASS** | 3-line selection + Ctrl+click Tab, and single-line variant |

Good news: **no hang and no crash in any row of this re-run**; every Ferrite process exited with code 0. Config was restored byte-identical after each batch that touched it. Direct fixes for ISS-65, 66 (behaviour), 67, 68, 72, 74, 75 are confirmed. Section D (ISS-69 YouTube) and SES-3 Quick-note ON (task 3) and T3-44 (task 4) are still waiting on task commits that are **not** in `git log`.

**Counts (40 rows, final results after supersession, §4):** **24 PASS / 4 FAIL / 1 PARTIAL / 11 BLOCKED** (0 SKIPPED).

| Set | Rows | PASS | FAIL | PARTIAL | BLOCKED |
|---|---:|---:|---:|---:|---:|
| A. Rendered-edit (ISS-64/65/68) | 10 | 7 | 3 | 0 | 0 |
| B. Raw / settings / popup (ISS-67/72/74/75) | 7 | 7 | 0 | 0 | 0 |
| C. Linux image paste (ISS-66) | 6 | 6 | 0 | 0 | 0 |
| D. YouTube (ISS-69) | 6 | 0 | 0 | 0 | 6 (build) |
| E. Fixtures / by-design | 5 | 1 | 1 | 1 | 2 (decision) |
| F. Observations + cargo test | 6 | 3 | 0 | 0 | 3 (build / WM / env) |
| **Total** | **40** | **24** | **4** | **1** | **11** |

BLOCKED breakdown: **7 by-build** (Section D ×6 + SES-3 Quick-note ON), **2 by-decision** (IND-9, CPY-3), **1 by-WM** (SI-1), **1 by-environment** (TTL-6a / Xvfb).

## 1. Automated gate

| Check | Result | Numbers |
|---|---|---|
| Correct commit / clean tree | PASS | HEAD `d1d4d15`, fix ancestor `8c9c2d5` present; `git status --short` empty. Task 1–4 cyclopsctl commits **absent** (see prep.md). |
| `cargo build --release --locked` | PASS | Finished in 2 m 42 s, exit 0. Binary 46 632 848 bytes. Evidence: `build-release.log` |
| `cargo test --locked` | PASS | **2235 passed / 0 failed / 2 ignored** (exit 0). `--list` count **2237** (= 2235 + 2 ignored). Windows baseline on the round-3 fix commit: **2246 passed** → Linux = 2246 − 11 `#[cfg(windows)]` = **2235** → **MATCH**. Closes the §6 cargo-test-count item. Ignored: `editor::ferrite::buffer::tests::test_large_file`, `editor::ferrite::history::tests::test_large_buffer_performance`. Evidence: `cargo-test.log` |
| Tools before first GUI row | PASS | `scrot` 1.12.1, `xclip` 0.13, `maim` 5.8.0-1, `xdotool`, ImageMagick 7.1.1-43, gdb 16.3; smoke OK |

## 2. Verdict against the doc's pass criteria

| Criterion (`rerun-v0.3.1-round3.md`) | Met? | Detail |
|---|---|---|
| 1. Every row is PASS or BLOCKED-by-input/environment/build | **No** | 4 FAIL (ISS-76…79), 1 PARTIAL (T3-44, known / task 4 — not a new FAIL). The BLOCKED rows qualify (build / decision / WM / environment). |
| 2. Tier 1 of the minimal checklist: all PASS or BLOCKED-by-input/environment | **No** | Of the Tier 1 rows re-checked here: T1-14/18/19/20 PASS; **T1-17 FAIL** (ISS-78). Other Tier 1 rows not listed in the round-3 gate passed in round 2 and were not required again. |
| 3. Screenshots + debug logs for FAILs; saved files for ISS-64/65/68 | Yes | All FAIL evidence under `shots/`, `logs/`, `files/`; ISS-68 fixtures byte-checked; ISS-65 diffs attached. |

Against the checklist "Tag gate (minimal)": rule 1 fails on T1-17, so **do not tag `v0.3.1`**. Suggested fix order before the next re-run: **ISS-78** (Tier 1 Rendered caret vs fenced code block), then land task 1 (ISS-69) / task 3 (SES-3) if still in scope, then ISS-76 (needs product decision), ISS-77, ISS-79.

## 3. Per-ISS summary (ISS-64 … ISS-75 from round 2, plus new)

Status key: **Fixed** = every re-checked row for that ISS is PASS. **Partially fixed** = some of the original rows now PASS, remaining failures filed as new issues. **Not fixed** = stated expectation still unmet. **Blocked** = not runnable on this build (task commit missing). **Deferred / by-design** = gate says BLOCKED-by-decision (PRD §8).

| ISS | Round-3 status | Rows → final | Notes |
|---|---|---|---|
| ISS-64 | **Partially fixed** | T1-18 PASS, ARR-4 PASS (+ probe past code block in Split PASS); T1-17 → **ISS-78**; ARR-2 → **ISS-77** (Ln only); T2-35 → **ISS-76** | With Preview padlock **unlocked**, blockquote entry and Split block walking work. Remaining failures are narrower and filed separately. A2 batch ran with the padlock locked by default and is **SUPERSEDED**. |
| ISS-65 | **Fixed** | T1-19 / UND-1 PASS; UND-3 PASS (gate steps, no mid-undo save) | No stray heading; one Ctrl+Z restores byte-for-byte; no `Skipping heading commit:` WARN. Observation: mid-undo Ctrl+S splits the undo group (not filed). |
| ISS-66 | **Fixed** (X11 behaviour) | T1-14, IMG-X1, IMG-X2, IMG-4, IMG-7, maim-owner T1-14 all PASS | Diagnostics: **no** `Image paste:` and **no** `Clipboard image read failed` lines; only `Smart paste: Saved clipboard image under assets/` (5 lines, quoted in §4.3). Task-2 code-fix commit still absent; §5.1 diagnostics + existing path are enough for PASS on this box. |
| ISS-67 | **Fixed** | T1-20 (3-line + Ctrl+click) PASS; single-line variant PASS | Needed Shift+End after Shift+Down ×2 to cover line 9 (gate fallback alone selected only 7–8). |
| ISS-68 | **Fixed** | Dirty-on-view ×3 fixtures PASS | No `*`, no save prompt, no `Content modified in rendered editor`; m165 wc -m 6376 / hashes unchanged. Fixture title now `# GitHub HTML Subset Test` (renders as H1). |
| ISS-69 | **Blocked** (task 1) | T2-36, VID-1, EXTRA-YT-2/4/6/7 → BLOCKED-by-build | Task-1 commit not in `git log`. Not run. WebKitGTK `2.54.0-1~deb13u3`. |
| ISS-70 | **Deferred / by-design** | CPY-3 → BLOCKED-by-decision | Clipboard = source markdown (PRD §8). |
| ISS-71 | **Deferred 0.3.2** | IND-9 → BLOCKED-by-decision | Tab in rendered list does nothing; app responsive; Raw unchanged (PRD §8). |
| ISS-72 | **Fixed** | EXTRA-SET-9a PASS; EXTRA-SET-9b PASS | Toasts visible ~4 s; `Config load toast:` INFO lines present; corrupt renamed; other settings kept. |
| ISS-73 | **Not fixed** | MMD-3 → **ISS-79** | New fan-out/fan-in fixture + `linkStyle … interpolate basis` still looks the same as without linkStyle (straight/angled edges). |
| ISS-74 | **Fixed** | SI-4 PASS; SI-4b PASS | Settings reach disk ~0.4 s after click; survive `pkill -9`. |
| ISS-75 | **Fixed** | SPL-3 PASS | Tooltip gone by the time the context menu opens; first suggestion fully visible. |

## 4. Full row table (final results)

Supersession rules applied (later / graded run wins):

- **A2 batch (T1-17, T1-18, ARR-2, ARR-4, T2-35):** run with the Preview padlock **locked by default** (unknown at the time). Those labels are **SUPERSEDED**. Authoritative runs: B (T1-17-confirm, ARR-2-confirm), C (T1-18, ARR-4, ARR-4 probe, T2-35), EF (T1-17-extended).
- **UND-3:** attempt 1 (Ctrl+S between typing and undo) left `return valuea` / `1 entries remaining` → observation only. Attempt 2 (gate steps, no save before undo) → **PASS**.
- **ARR-2:** B batch caret movement PASS, but status-bar `Ln` stayed `Ln 1, Col 1` → final **FAIL (ISS-77)** on the Ln part of the expected result.
- **SES-3 Quick-note ON:** results-EF labelled FAIL; graded **BLOCKED-by-build** (task 3 not landed), with the Alt+F4 observation kept.

### 4.1 Section A — Rendered-edit (ISS-64, 65, 68)

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| ISS-68 github_html | ISS-68 | **PASS** | A1 | shots/ISS68-html-rendered-H1.png, ISS68-html-title.png; logs/ISS68-html.log | No `*`; H1 renders; wc/hash unchanged; no save prompt |
| ISS-68 m165 | ISS-68 | **PASS** | A1 | shots/ISS68-m165-*.png; logs/ISS68-m165.log | wc -m **6376**; hash unchanged; grep `Content modified in rendered editor` = 0 |
| ISS-68 git_graphs | ISS-68 | **PASS** | A1 | shots/ISS68-git-*.png; logs/ISS68-git.log | Same |
| T1-19 / UND-1 | ISS-65 | **PASS** | A1 | files/UND1-step{1,2,3,4}-*; shots/ISS65-UND1-*.png; logs/UND1.log | `return valuex` on line 42; one Ctrl+Z byte-identical; redo only `x`; no `Skipping heading commit:` |
| UND-3 | ISS-65 | **PASS** | A1 (attempt 2) | files/UND3b-*; shots/ISS65-UND3b-*.png; logs/UND3b.log | All 5 chars present; one Ctrl+Z → empty diff. Attempt 1 (with Ctrl+S) → observation §6 |
| T1-17 | ISS-64 → **ISS-78** | **FAIL** | B confirm + EF extended | shots/B-T1-17c-*.png, EF-T1-17-*.png; files/B-T1-17c.*, EF-T1-17.*; logs/B-T1-17c.log, EF-T1-17.log | Rendered-only unlocked: caret stops at paragraph above Rust fence; presses 16–20 no-op. A2 SUPERSEDED |
| T1-18 | ISS-64 | **PASS** | C | shots/C-T1-18-*.png; files/C-T1-18.*; logs/C-T1-18.log | Padlock unlocked; `x` inside blockquote (`> xA quoted…`); A2 SUPERSEDED |
| ARR-2 | ISS-64 → **ISS-77** | **FAIL** | B | shots/B-ARR2c-*.png; files/B-ARR2c.*; logs/B-ARR2c.log | Caret moves to next visual row (`Z` on row 3); **status bar stays `Ln 1, Col 1`** |
| ARR-4 | ISS-64 | **PASS** | C (+ probe) | shots/C-ARR-4-*.png, C-ARR-4-probe-*.png; files/C-ARR-4.*, C-ARR-4-probe.*; logs/C-ARR-4.log, C-ARR-4-probe.log | Walks blocks; Raw Ln unchanged. Probe: caret passes Rust code block in **Split** |
| T2-35 | ISS-64 → **ISS-76** | **FAIL** | C | shots/C-T2-35-*.png; files/C-T2-35.*; logs/C-T2-35.log | Lock ON: `Q` not inserted, Raw edits OK; **no caret / ArrowDown does not move between blocks**. May be by design — Ola's call. A2 SUPERSEDED |

### 4.2 Section B — Raw / settings / popup

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| T1-20 (3-line + Ctrl+click) | ISS-67 | **PASS** | B | shots/B-T1-20-*.png; files/B-T1-20-tab.*, -undo.*; logs/B-T1-20.log | Shift+End after Shift+Down ×2 to cover line 9; undo byte-identical |
| T1-20 single-line | ISS-67 | **PASS** | B | shots/B-T1-20-single-*.png; files/B-T1-20-single.* | Whole line indented, text intact |
| EXTRA-SET-9a | ISS-72 | **PASS** | B | shots/B-SET9a-2.png (toast); logs/B-SET9a.log | Toast + `Config load toast: unreadable config renamed to …corrupt-20261109-162010` |
| EXTRA-SET-9b | ISS-72 | **PASS** | B | shots/B-SET9b-2.png; logs/B-SET9b.log | Toast + `Config load toast: 1 invalid field(s) reset…`; other settings kept |
| SI-4 | ISS-74 | **PASS** | B | shots/B-SI4-after-click.png; files/B-SI4-config-after.json; logs/B-SI4.log | `"show_line_numbers": false` ~0.4 s after click; survives pkill -9 |
| SI-4b | ISS-74 | **PASS** | B | shots/B-SI4b-after-click.png; logs/B-SI4b.log | `"spellcheck_enabled": true` persisted |
| SPL-3 | ISS-75 | **PASS** | B | shots/B-SPL3-hover.png, B-SPL3-rc-0ms.png, B-SPL3-rc-500ms.png; logs/B-SPL3.log | Tooltip gone; first suggestion visible |

Config restored byte-identical from `config-backup-set9/` after this batch (`cmp`).

### 4.3 Section C — Linux image paste (ISS-66)

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| T1-14 (xclip) | ISS-66 | **PASS** | C | shots/C-T1-14-*.png; files/C-T1-14.*; work/paste/assets/20261109-153203-clipboard.png; logs/C-T1-14.log, C-T1-14-TARGETS.txt | `TARGETS` = `image/png`; toast `Image added to assets` |
| IMG-X1 | ISS-66 | **PASS** | C | shots/C-IMG-X1-*.png; logs/C-IMG-X1.log | Toast `Save the document first to store images`; **no** paste diagnostic line |
| IMG-X2 | ISS-66 | **PASS** | C | shots/C-IMG-X2-*.png; files/C-IMG-X2.*; work/paste/assets/20261109-153345-clipboard{,-1}.png; logs/C-IMG-X2.log | Two distinct images; one toast; no spam |
| IMG-4 | ISS-66 | **PASS** | C | shots/C-IMG-4-after-undo.png; files/C-IMG-4.*; logs/C-IMG-4.log | One Ctrl+Z removes Markdown; asset remains on disk |
| IMG-7 | ISS-66 | **PASS** | C | shots/C-IMG-7-*.png; files/C-IMG-7.*; work/paste-img7/assets/…; logs/C-IMG-7.log | Raw paste with Split lock closed; preview shows red image |
| T1-14 (maim owner) | ISS-66 | **PASS** | C | shots/C-T1-14-maim-*.png; files/C-T1-14-maim.*; work/paste-maim/assets/…; logs/C-T1-14-maim.log, C-T1-14-maim-TARGETS.txt | Same behaviour; `TARGETS` = `image/png` |

#### Exact ISS-66 diagnostics (verbatim)

No log lines containing `Image paste:` were emitted. No `Clipboard image read failed` line was emitted. Successful saved-document pastes reported only:

```text
[2026-11-09T15:32:03Z DEBUG ferrite::app::input_handling] Smart paste: Saved clipboard image under assets/
[2026-11-09T15:33:45Z DEBUG ferrite::app::input_handling] Smart paste: Saved clipboard image under assets/
[2026-11-09T15:33:45Z DEBUG ferrite::app::input_handling] Smart paste: Saved clipboard image under assets/
[2026-11-09T15:34:56Z DEBUG ferrite::app::input_handling] Smart paste: Saved clipboard image under assets/
[2026-11-09T15:35:52Z DEBUG ferrite::app::input_handling] Smart paste: Saved clipboard image under assets/
```

(Order: T1-14 xclip; IMG-X2 first; IMG-X2 second; IMG-7; T1-14 maim.)

### 4.4 Section D — YouTube (ISS-69) — not run

| Row | Final | Notes |
|---|---|---|
| T2-36 | **BLOCKED-by-build** | Task-1 commit not in `git log` |
| VID-1 | **BLOCKED-by-build** | Same |
| EXTRA-YT-2 | **BLOCKED-by-build** | Same |
| EXTRA-YT-4 | **BLOCKED-by-build** | Same |
| EXTRA-YT-6 | **BLOCKED-by-build** | Same |
| EXTRA-YT-7 | **BLOCKED-by-build** | Same |

`libwebkit2gtk-4.1-0` = **2.54.0-1~deb13u3** (recorded for when task 1 lands).

### 4.5 Section E — Fixtures / by-design

| Row | ISS | Final | Batch | Evidence | Notes |
|---|---|---|---|---|---|
| MMD-3 | ISS-73 → **ISS-79** | **FAIL** | EF | shots/EF-MMD-3-with-linkstyle.png, -without-linkstyle.png, -comparison-zoom.png; files/EF-MMD-3-input.diff; logs/EF-MMD-3.log | Edges look the same with/without `linkStyle 0,1,2,3 interpolate basis`; no visible curve |
| HTML-6 | ISS-33 | **PASS** | EF | shots/EF-HTML-6-rendered.png, -crop.png; logs/EF-HTML-6.log | H1 renders; adjacent `<div align>` show their own text |
| T3-44 | ISS-33 / task 4 | **PARTIAL** | EF | same shots; logs/EF-T3-44.log | Known: center/right blocks still left-aligned. **Not a new FAIL** |
| IND-9 | ISS-71 | **BLOCKED-by-decision** | EF | shots/EF-IND-9-*.png; files/EF-IND-9.*; logs/EF-IND-9.log | Tab does nothing; Raw unchanged. PRD §8 / deferred 0.3.2 |
| CPY-3 | ISS-70 | **BLOCKED-by-decision** | EF | shots/EF-CPY-3-*.png; files/EF-CPY-3-clipboard.txt; logs/EF-CPY-3.log | Clipboard = source markdown. PRD §8 / by design 0.3.1 |

### 4.6 Section F — Observations + cargo test

| Row | Final | Batch | Evidence | Notes |
|---|---|---|---|---|
| SES-3 Quick-note OFF | **PASS** | EF | shots/EF-SES-3-off-*.png; logs/EF-SES-3-off.log | × and Alt+F4 both show Unsaved Changes prompt |
| SES-3 Quick-note ON | **BLOCKED-by-build** | EF | shots/EF-SES-3-on-*.png; logs/EF-SES-3-on.log | Task 3 not landed. Observation: × closes without prompt (expected for Quick-note ON); **Alt+F4 silently leaves `Untitled*` open with no prompt** |
| SI-3 | **PASS** | EF | shots/EF-SI-3-*.png; files/EF-SI-3-{before,after}-metadata.txt; logs/EF-SI-3.log | Secondary `--new-instance` leaves session.json / recovery untouched |
| SI-1 | **BLOCKED-by-WM** | EF | shots/EF-SI-1-after-forward.png; files/EF-SI-1-xprop-before.txt, EF-SI-1-xprop-after.txt; logs/EF-SI-1.log | Forwarded open while minimized; window stayed minimized. xprop before: `_NET_WM_STATE(ATOM) = _NET_WM_STATE_FOCUSED`; after: `_NET_WM_STATE(ATOM) = _NET_WM_STATE_HIDDEN` |
| TTL-6a | **BLOCKED-by-environment** | EF | shots/EF-TTL-6a-*.png; files/EF-TTL-6a-geometry.txt; logs/EF-TTL-6a.log | Xvfb: EWMH maximized flags cleared but geometry stayed 1920×1159 at (0,0) |
| `cargo test` count | **PASS** | prep | cargo-test.log; prep.md | **2235** = 2246 − 11; closes §6 item |

Config restored byte-identical from `config-backup-EF/` after this batch (`cmp` / `diff -r` empty).

## 5. New issues (ISS-76 …)

The highest ID from round 2 is ISS-75, so new issues start at **ISS-76**. Nothing was filed on GitHub and the round-3 PRD was not edited. Severity: Tier 1 FAIL = **blocker**; otherwise feature-not-working = major/minor per brief; product-decision rows noted.

| ID | Title | Severity | Rows |
|---|---|---|---|
| ISS-76 | Locked preview (T2-35): no caret and ArrowDown does not move between blocks | **minor** (needs product decision — may be by design) | T2-35 |
| ISS-77 | Status bar `Ln` / `Col` does not track the Rendered caret | **minor** | ARR-2 |
| ISS-78 | Rendered-only ArrowDown cannot pass a fenced code block | **blocker** (Tier 1) | T1-17 |
| ISS-79 | Mermaid `linkStyle … interpolate basis` has no visible curve effect (fan-out/fan-in fixture) | **minor** | MMD-3 |

### ISS-76 — Locked preview: no caret / ArrowDown does not move between blocks
- **Row:** T2-35. Severity **minor** (or by-design — **decision for Ola**).
- **Repro:** 1. Open a copy of `test_find_arrow_nav.md`, **Split**. 2. Confirm the Preview padlock bottom-right is **locked** (closed). 3. Click a Rendered paragraph. 4. Press ArrowDown. 5. Type `Q`. 6. Confirm Raw is still editable.
- **Expected (gate):** Arrow moves the caret between blocks; `Q` is **not** inserted; Raw edits still work.
- **Actual:** No visible caret; ArrowDown shows no caret/viewport movement between blocks; `Q` correctly ignored; Raw remains editable (temporary `R` then Ctrl+Z restored the fixture).
- **Evidence:** `shots/C-T2-35-*.png`; `files/C-T2-35.md`, `C-T2-35.diff` (empty); `logs/C-T2-35.log`. Clean exit.

### ISS-77 — Status bar `Ln` does not follow the Rendered caret
- **Row:** ARR-2. Severity **minor**.
- **Repro:** 1. `test_find_arrow_nav.md`, **Rendered**, padlock unlocked, status bar visible (maximised window). 2. Click the 2nd visual row of the wrapped Section 4 paragraph. 3. Press ArrowDown. 4. Optionally type a marker character.
- **Expected:** Caret moves to the next visual row **and** status bar `Ln` changes.
- **Actual:** Caret correctly moves to the next visual row (`Z` landed on row 3, same column). Status bar stayed **`Ln 1, Col 1`** before and after ArrowDown and after typing in Rendered.
- **Evidence:** `shots/B-ARR2c-before.png`, `B-ARR2c-after-down.png`, `B-ARR2c-after-Z.png`; `files/B-ARR2c.diff`; `logs/B-ARR2c.log`. Clean exit. No arrow-nav debug lines in the log.

### ISS-78 — Rendered-only caret cannot pass a fenced code block with ArrowDown
- **Row:** T1-17 (Tier 1). Severity **blocker**.
- **Repro:** 1. Copy `test_md/test_find_arrow_nav.md`. 2. **Rendered**, unlock the Preview padlock. 3. Click in an early paragraph (e.g. `The ferrite crystal sat on the shelf.` or `## Section 1`). 4. Press ArrowDown repeatedly (≥ 15–20 presses, ≥ 100 ms apart). 5. Observe whether the caret reaches the Section 4 multi-line paragraph past the Rust fence.
- **Expected:** Caret walks down and reaches the off-screen multi-line Section 4 paragraph; typing `X` lands there; heading unchanged.
- **Actual (Rendered-only):** Caret walks by visual rows down to **`The word ferrite appears here again.`** (the paragraph immediately above the Rust code block) and **stops**. Further ArrowDown presses (16–20 in the extended run) do nothing. B confirm: typed `X` landed at the start of that same line (`XThe word ferrite appears here again.`). Saved file otherwise intact.
- **Contrast (Split):** ARR-4 probe from the same paragraph, ArrowDown ×3 + `V`, landed past the Rust fence on a Section 4 line (`Vdeserunt mollit…`). So Split can pass the block; Rendered-only cannot.
- **Also:** no runtime arrow-navigation debug lines appeared in any ISS-64 log (only the fixture filename in startup/session lines).
- **Evidence:** `shots/B-T1-17c-*.png`, `shots/EF-T1-17-after-{15,16c,20}.png`, `EF-T1-17-raw-after.png`; `files/B-T1-17c.diff`, `B-T1-17c-saved.md`, `EF-T1-17.diff` (empty on extended — no `X` typed); `logs/B-T1-17c.log`, `EF-T1-17.log`. Clean exits. A2 T1-17 (no `X` landed at all) SUPERSEDED — that run had the padlock locked / focus issues.

### ISS-79 — Mermaid `linkStyle interpolate basis` has no visible effect
- **Row:** MMD-3. Successor to ISS-73 / ISS-57. Severity **minor**.
- **Repro:** 1. Open `test_md/test_mermaid_v031.md` §3 (fan-out/fan-in `A→B, A→C, B→D, C→D` with `linkStyle 0,1,2,3 interpolate basis`). 2. Rendered; screenshot. 3. Same diagram without the `linkStyle` line; screenshot. 4. Compare.
- **Expected:** Edges visibly curved and **different** from the no-linkStyle version.
- **Actual:** Edges look the same (straight/angled segments); zoomed comparison nearly identical; no Mermaid linkStyle diagnostic in the log.
- **Evidence:** `shots/EF-MMD-3-with-linkstyle.png`, `EF-MMD-3-without-linkstyle.png`, `EF-MMD-3-comparison-zoom.png`; `files/EF-MMD-3-input.diff`; `logs/EF-MMD-3.log`. Clean exit.

## 6. Notable observations (not filed as issues)

| Item | What | Suggested follow-up |
|---|---|---|
| UND-3 + Ctrl+S | Typing `abcde` then **Ctrl+S** then Ctrl+Z undid only `bcde` (`return valuea` left; log `Undo performed, 1 entries remaining`). Without the save, one Ctrl+Z clears all five. Likely Ctrl+S breaks the undo group. | Confirm whether save intentionally commits the undo group; if yes, note it in the gate so testers don't save mid-row. |
| Code-block Edit button | Clicking the code **text** in Split preview / Rendered gives no caret; typed chars ignored. Typing works only after the block's **Edit** button. | Document in the gate / checklist ("click Edit, then end of line"). |
| Gate line numbers (T1-19) | `x` lands on line **42** (`return value`); line 43 is the closing fence. Gate text saying "line 43" is off by one. | Fix the gate doc line numbers. |
| Preview padlock default | The Preview padlock **starts locked**. ArrowDown then scrolls / ignores typing / shows no caret — easy to mis-grade ISS-64 rows (A2 batch). | Gate doc should say: unlock before editable Rendered steps; confirm padlock state in every ARR/T1-17/18/T2-35 screenshot. |
| SES-3 Quick-note ON + Alt+F4 | With Quick-note ON, × closes the dirty Untitled secondary window (OK). **Alt+F4** leaves `Untitled*` open with no prompt. | Re-test when task 3 lands; may be the same family as the round-2 Untitled-only observation. |
| Computer screenshot lag | Computer-use frames often lagged one action behind; **scrot** shots are the evidence of record. | Harness note only. |
| No arrow-nav log lines | Despite `--log-level debug`, no runtime arrow/caret navigation event lines appeared in any ISS-64 log. | Useful if the fix added `debug!` lines that should fire — they did not on this path. |
| SI-1 xprop before | `EF-SI-1-xprop-before.txt` shows `_NET_WM_STATE_FOCUSED` (not HIDDEN). After forward: `_NET_WM_STATE_HIDDEN`. Results text says the window was minimized before forward; the before-capture may have been taken before minimize, or focus flipped. After-state is unambiguous. | Re-capture before-state on next SI-1 run if needed. |

## 7. Environment notes and harness limits

- **Display:** Xvfb `:2` 1920×1200, xfwm4 (grabs Alt+click / Alt+Space), Mesa llvmpipe. Maximised Ferrite window used so the status bar is visible.
- **WebKitGTK:** `libwebkit2gtk-4.1-0` **2.54.0-1~deb13u3** (Section D not run on it this round).
- **Clipboard owners for ISS-66:** `xclip` offering only `image/png`, and `maim -g 100x100+0+0 | xclip -selection clipboard -t image/png` (also `image/png` only). Both PASS.
- **apt:** still broken behind the sandbox proxy; packages installed via `curl` + `dpkg -i` (including freshly installed `maim` 5.8.0-1).
- **xdotool:** Ctrl chords held ~0.4 s; ArrowDown presses ≥ 100–180 ms apart.
- **Config:** backed up under `rerun3/config-backup-{start,set9,EF,round2-H-ref}/`; restored with `cmp` after B and EF. No Ferrite process left running after each batch.
- **End state:** no source changes; nothing committed, pushed, or filed. Original full-resolution PNGs remain in `rerun3/shots/`; the shots zip carries downscaled JPEGs (max width 1280, q=82) to stay under the attachment size limit.

## 8. Attachments

| File | Size | Contents |
|---|---|---|
| `rerun3/qa-rerun3-report-v0.3.1-linux.md` | (this file) | Round 3 Linux QA report |
| `rerun3/rerun3-shots.zip` | **13.16 MB**, 193 files | Downscaled JPEG copies of `shots/*` (skipping webp duplicate of `B-ARR2c-before`). Originals: `rerun3/shots/` (194 files, ~51 MB) |
| `rerun3/rerun3-logs.zip` | **5.43 MB**, 77 entries | All `logs/*` from this re-run, plus `results-A1.md` … `results-EF.md`, `prep.md`, `build-release.log`, `cargo-test.log`, `fixture-checksums.txt` |
| `rerun3/rerun3-files.zip` | **55 KB**, 82 entries | `files/*` (saved markdown, diffs, xprop, clipboard dump, metadata) + `work/paste/`, `work/paste-img7/`, `work/paste-maim/` (including pasted `assets/*.png`) |
