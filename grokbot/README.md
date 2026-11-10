# GrokBot QA material — v0.3.1 (Linux)

Everything in this folder comes from **four** autonomous QA runs by GrokBot on
Debian 13, X11 + Xvfb, xfwm4, Mesa llvmpipe (no GPU), synthetic xdotool input:

1. **October (round 1 + 2 source)** against `8e6ef508` — 198 rows, ISS-1…63.
2. **November (round-2 re-run)** against `221b6ae` (HEAD `9effd76`), driven by
   `New/rerun-v0.3.1-round2.md` — 98 rows, 65 PASS / 23 FAIL / 7 PARTIAL /
   2 BLOCKED / 1 DEFERRED, new issues ISS-64…75. The two Linux crashers (ISS-1
   deadlock, ISS-2 GDK abort) are gone; the rendered-edit cluster came back in a
   new shape.
3. **November (round-3 re-run)** against `8c9c2d5` (HEAD `d1d4d15`), driven by
   `november/rerun-v0.3.1-round3.md` — 40 rows, 24 PASS / 4 FAIL / 1 PARTIAL /
   11 BLOCKED, new issues ISS-76…79. ISS-65/66/67/68/72/74/75 confirmed fixed,
   no hang or crash. The one Tier-1 FAIL (T1-17 / ISS-78) turned out to be a
   press-count error in the gate, not a product bug; ISS-76/77/79 and the SES-3
   Alt+F4 observation were real.
4. **November (round-4 re-run)** against `e56243c` (HEAD `1ad150b`), driven by
   `november/rerun-v0.3.1-round4.md` — 33 rows, 22 PASS / 0 FAIL / 1 PARTIAL /
   10 BLOCKED, **no new issues**. ISS-76/77/78/79 and SES-3a–h confirmed fixed;
   `cargo test --locked` 2249 / 0 / 2. The 10 BLOCKED rows are the six ISS-69
   YouTube rows (task 1 not built), CPY-3 / IND-9 (by decision), SI-1 / TTL-6a
   (Xvfb). SES-3d needed `xdg-desktop-portal` + zenity installed on the box
   before rfd could open Save As; it then passed.

## Layout

| Path | What it is | Use it for |
|------|------------|------------|
| `New/qa-report-v0.3.1-linux.md` | **October canonical report.** Batches 1–12, 198 rows, ISS-1…63, **Status** column filled for round 1, `5eec844` and round 2. | Issue history for ISS-1…63 |
| `New/rerun-v0.3.1-round2.md` | Round-2 hand-off GrokBot executed in November. **Superseded** by the round-3 doc for the next run. | Reading what the November run was asked to verify |
| `New/qa-notes/`, `New/qa-shots-*.zip`, `New/qa-logs-*.zip` | October raw notes, 406 screenshots, 246 logs. | Evidence for ISS-1…63 |
| `november/qa-rerun2-report-v0.3.1-linux.md` | **November canonical report.** §4 full row table, §5 ISS-64…75 with a **Status** column (round-3 triage), §6 PARTIALs with Status, §7 harness limits. | Triage of ISS-64…75; what still needs a re-run |
| `november/rerun-v0.3.1-round3.md` | Round-3 hand-off GrokBot executed. **Superseded** by the round-4 doc. Section D (ISS-69) is still the reference for the YouTube rows. | Reading what the round-3 run was asked to verify |
| `november/rerun3/qa-rerun3-report-v0.3.1-linux.md` | **Round-3 canonical report.** §0 verdict, §3 per-ISS status, §4 row table, §5 ISS-76…79, §6 observations, §7 harness limits. | Triage of ISS-76…79 |
| `november/rerun3/logs/`, `shots/`, `files/` | The three round-3 zips unpacked (each has one nested folder of the same name). `logs/results-*.md` are the per-batch notes. The zips, `shots/` and `logs/` are git-ignored; `files/` (77 small saved files and diffs) and the report are tracked. | Root-causing a round-3 row by its `shots/…` / `logs/…` / `files/…` name |
| `november/rerun-v0.3.1-round4.md` | Round-4 hand-off GrokBot executed. A arrow navigation / status bar / locked preview (ISS-76/77/78), B Mermaid curve (ISS-79), C secondary window close (SES-3), D sanity rows, E rows still waiting on a build. Ends with the triage table for the round-3 report. Section E is still the reference for the ISS-69 YouTube rows. | Reading what the round-4 run was asked to verify |
| `november/rerun4/qa-rerun4-report-v0.3.1-linux.md` | **Round-4 canonical report.** §0 verdict (0 FAIL), §3 per-ISS status, §4 row table, §6 observations (Return reaches the editor while the close dialog is open; Mermaid Source toggle truncated at 200 %), §7 environment incl. the portal install. | Confirming ISS-76…79 / SES-3; what is still open before the tag |
| `november/rerun4/logs/`, `shots/`, `files/`, `results-*.md`, `prep.md` | The three round-4 zips unpacked. `results-A…D.md` are the per-batch notes; `prep.md` has the build / test / fixture checks. The zips, `shots/` and `logs/` are git-ignored; `files/` (saved files, diffs, greps) and the `.md` files are tracked. | Root-causing a round-4 row by its `shots/…` / `logs/…` / `files/…` name |
| `november/triage-prompt.md` | The prompt that produced the round-3 triage (this README, the PRD, the rerun doc). | Provenance |
| `november/rerun2-shots.zip`, `november/rerun2-logs.zip` | November evidence archives (git-ignored). | — |
| `november/rerun2/` | GrokBot's per-batch notes (`results-*.md`), the UND-3 diff and corrupted file, env-restore notes — the November equivalent of `New/qa-notes/`. Tracked (its `.webp` and `shots-G/` are not). | When the summary row in the report is not enough |
| `november/shots/`, `november/logs/` | The zips **unpacked** (git-ignored). | Root-causing an ISS-64…75 row by the `shots/…` / `logs/…` name in the report |
| `superseded/batches-1-4/`, `superseded/batches-1-6/` | Earlier snapshots of the October run. | Nothing — safe to delete |

Git-ignored: `grokbot/**/*.zip`, `grokbot/**/shots/`, `grokbot/**/shots-*/`,
`grokbot/**/*.png`, `grokbot/**/*.webp`, and `logs` (global rule). The `.md`
reports, hand-offs, notes and this README are tracked.

## What GrokBot delivered (assessment)

- **October:** complete minimal checklist + non-overlapping total-test rows +
  Settings/YouTube extras; every FAIL with repro, expected/actual, screenshot,
  log and (for the crashes) a correct gdb source pointer.
- **November:** executed the round-2 hand-off row by row, kept the FAIL/PASS
  discipline (re-graded HTML-6 to PASS when the evidence said so, filed ISS-68
  separately instead of folding it into a row), survived a mid-run box restart
  with a documented environment restore, and wrote §7 harness limits that made
  the triage possible (xdotool chord timing, selection drag = one line in T1-20,
  screenshots lagging a frame, `xclip` as the only image clipboard owner).
- **Gaps (by design of the run):** Linux X11 only, no GPU, no real display,
  synthetic drag unreliable, Alt+click grabbed by xfwm4, network through a proxy
  (YouTube rows need confirmation on a desktop), Batch-A screenshots lost (no
  `scrot` yet).

## Where the fixes are

| Source of fixes | Commit(s) | Covers |
|-----------------|-----------|--------|
| Round 1 bucket A/B (direct) | `5325e9b`, `97347d3` | ISS-1/3/4/8/10/11/17/20/27, ISS-2/5 |
| Round 1 orchestrator (buckets C/D/E) | `51f5a3a` | ISS-6/7/9/12/14/15/16/18/19/21/22/23/24/25/26/28/29 |
| Round 2 direct fixes | `5eec844` | ISS-35/37/38/43/45/46/53/55/57/58/62/63 |
| Round 2 orchestrator (tasks 1–33) | `221b6ae` (gate doc `9effd76`) | ISS-30/32/33/34/36/39/40/41/42/44/47/48/49/50/51/52/54/56/59/60/61 + SI-3/4, GIT-3, CSV-4, SPL-3, HTML-4 |
| **Round 3 direct fixes** | **`8c9c2d5`** (triage docs `08aabd0`+) | ISS-64/65/67/68/72/74/75 with regression tests; ISS-66 diagnostics; ISS-73 fixture; `test_github_html.md` typo. Docs: `docs/technical/markdown/rendered-session-scope-and-seed.md` and updates to `indent-selection.md`, `settings-debounced-save.md`, `config-load-toasts.md`, `diagnostic-hover-popup-guard.md` |
| Round 3 orchestrator | `docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md` §6 (not yet parsed) | task 1 ISS-69 Linux embed origin (**still open**); task 2 ISS-66 follow-through (**not needed** — all X11 paste rows passed in round 3); task 3 SES-3 window close (**done directly in round 4**); task 4 = 0.3.2 bundle (ISS-70/71, T3-44, HTML-4, EXTRA-SET-7, MMD-10, T1-4b, T1-13) |
| **Round 4 direct fixes** | **`e56243c`** | ISS-76 (locked preview scrolls with Up/Down), ISS-77 (status bar follows the rendered caret), ISS-78 (gate/checklist press count + `Rendered arrow nav` debug line + fixture test), ISS-79 (S-curve for straight `interpolate basis` edges), SES-3 (window-manager close of a secondary window; window close prompts for untitled notes). Docs: `rendered-arrow-navigation.md`, `preview-lock.md`, `flowchart-linkstyle-interpolate.md`, `multi-window-implementation.md`, `quick-note-workflow.md` |

Decisions (by design / deferred): round-3 PRD §8 — ISS-70, ISS-71, ISS-73,
UX-9, MMD-1, T1-8, SES-6.

## Next steps

1. ~~Commit the round-3 fixes~~ — done in `8c9c2d5` (`cargo test --bin ferrite`: 2246 passed on Windows).
2. ~~Round-3 GrokBot run~~ — done, report in `november/rerun3/`.
3. Human pass on Windows for the round-4 changes: rows **A3** and **B13** of
   `docs/technical/platform/v0.3.1-quick-manual-test.md`, plus a look at
   `test_md/test_mermaid_v031.md` §3. The secondary-window close (B13) has no
   automated test.
4. ~~Commit the round-4 fixes~~ — done in `e56243c`.
5. Decide ISS-69 (round-3 PRD task 1, Linux YouTube Error 153): fix before the
   tag, or ship 0.3.1 with it as a known Linux issue. This is the only Linux
   QA item still standing between `1ad150b` and the tag.
6. ~~Hand `november/rerun-v0.3.1-round4.md` + the fix commit to GrokBot~~ —
   done, report in `november/rerun4/` (0 FAIL, no ISS-80+).
7. If ISS-69 is fixed: one more GrokBot pass on the Section E YouTube rows only
   (T2-36, VID-1, EXTRA-YT-2/4/6/7) — the box now has `libwebkit2gtk-4.1-0`
   2.54.0 and the portal stack installed. If it is waived: nothing further on
   Linux for 0.3.1.
8. Round-4 §6 observations, not filed: the Unsaved Changes dialog is a plain
   egui `Window`, so Return / typing still reach the editor behind it (all
   variants: tab close, exit, window close — pre-existing, not a round-4
   regression); Mermaid header **Source** toggle truncates at 200 % zoom.
   Candidates for 0.3.2 polish unless the human wants them in 0.3.1.
