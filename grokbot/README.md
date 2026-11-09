# GrokBot QA material — v0.3.1 (Linux)

Everything in this folder comes from **two** autonomous QA runs by GrokBot on
Debian 13, X11 + Xvfb, xfwm4, Mesa llvmpipe (no GPU), synthetic xdotool input:

1. **October (round 1 + 2 source)** against `8e6ef508` — 198 rows, ISS-1…63.
2. **November (round-2 re-run)** against `221b6ae` (HEAD `9effd76`), driven by
   `New/rerun-v0.3.1-round2.md` — 98 rows, 65 PASS / 23 FAIL / 7 PARTIAL /
   2 BLOCKED / 1 DEFERRED, new issues ISS-64…75. The two Linux crashers (ISS-1
   deadlock, ISS-2 GDK abort) are gone; the rendered-edit cluster came back in a
   new shape.

## Layout

| Path | What it is | Use it for |
|------|------------|------------|
| `New/qa-report-v0.3.1-linux.md` | **October canonical report.** Batches 1–12, 198 rows, ISS-1…63, **Status** column filled for round 1, `5eec844` and round 2. | Issue history for ISS-1…63 |
| `New/rerun-v0.3.1-round2.md` | Round-2 hand-off GrokBot executed in November. **Superseded** by the round-3 doc for the next run. | Reading what the November run was asked to verify |
| `New/qa-notes/`, `New/qa-shots-*.zip`, `New/qa-logs-*.zip` | October raw notes, 406 screenshots, 246 logs. | Evidence for ISS-1…63 |
| `november/qa-rerun2-report-v0.3.1-linux.md` | **November canonical report.** §4 full row table, §5 ISS-64…75 with a **Status** column (round-3 triage), §6 PARTIALs with Status, §7 harness limits. | Triage of ISS-64…75; what still needs a re-run |
| `november/rerun-v0.3.1-round3.md` | **Hand-off for the next GrokBot run.** Only rows needing re-verification, grouped: A rendered session, B raw/settings/popup, C ISS-66 Linux diagnostics, D ISS-69 (after task 1), E fixture / by-design rows, F §6 re-tests. | Give this to GrokBot with the round-3 fix commit |
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
| Round 3 orchestrator | `docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md` §6 (not yet parsed) | task 1 ISS-69 Linux embed origin; task 2 ISS-66 follow-through; task 3 SES-3 Untitled-only window close; task 4 = 0.3.2 bundle (ISS-70/71, T3-44, HTML-4, EXTRA-SET-7, MMD-10, T1-4b, T1-13) |

Decisions (by design / deferred): round-3 PRD §8 — ISS-70, ISS-71, ISS-73,
UX-9, MMD-1, T1-8, SES-6.

## Next steps

1. ~~Commit the round-3 fixes~~ — done in `8c9c2d5` (`cargo test --bin ferrite`: 2246 passed on Windows).
2. Human pass on Windows: Part A of
   `docs/technical/platform/v0.3.1-quick-manual-test.md` (round-3 rows A1–A12).
3. Parse the round-3 PRD into cyclopsctl; tasks 1–3 gate 0.3.1, task 4 may go
   to a `v0.3.2` tag.
4. Hand `november/rerun-v0.3.1-round3.md` + the fix commit to GrokBot. Section D
   only after task 1 lands.
5. Any new FAIL becomes ISS-76+, not an edit of the round-3 PRD.
