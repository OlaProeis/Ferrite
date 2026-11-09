# GrokBot QA material — v0.3.1 (Linux)

Everything in this folder comes from **one** autonomous QA run by GrokBot against
commit **`8e6ef508`** (`0.3.1-experimental`, 2026-10-08/09) on Debian 13, X11 +
Xvfb, xfwm4, Mesa llvmpipe (no GPU), synthetic xdotool input. **No build with the
fixes has been tested by GrokBot yet** — every `fixed … GrokBot re-run pending`
status in the report is still pending.

## Layout

| Path | What it is | Use it for |
|------|------------|------------|
| `New/qa-report-v0.3.1-linux.md` | **Canonical report.** Batches 1–12, 198 rows, ISS-1…63, with the **Status** column filled in for round 1 (Tier A/B), direct fixes `5eec844` and round 2 (tasks 1–33). | Triage, issue history, what still needs a re-run |
| `New/rerun-v0.3.1-round2.md` | **Hand-off for the next GrokBot run** (or a human on Linux): which rows to re-run per ISS, the fix commit, and the expected result. | Give this to GrokBot together with the current HEAD |
| `New/qa-notes/` | GrokBot's raw per-batch notes (`results-batch1…12.md`), its `gui-test-plan.md` and `report-format.md`, unpacked from `qa-logs-2.zip`. | When the summary row in the report is not enough |
| `New/qa-shots-1…5.zip` | All 406 screenshots (`shots/*.png`), each file exactly once. | Evidence lookup by the `shots/…` name in a row |
| `New/qa-logs-1.zip`, `New/qa-logs-2.zip` | All 246 debug logs, gdb hang backtraces (`logs/*-hang-bt.txt`), EOL/hex checks. | Root-causing a row |
| `superseded/batches-1-4/` | Earlier snapshot of the *same* run (batches 1–4 only, ISS-1…29). Its Tier B status cells were ported into the canonical report. | Nothing — kept for provenance; safe to delete |
| `superseded/batches-1-6/` | Earlier snapshot (batches 1–6, ISS-1…35). Strict subset of `New/`. | Nothing — safe to delete |

The `.zip` files are git-ignored (`grokbot/**/*.zip`); only the `.md` files belong
in the repo. Logs mention `logs/ferrite-debug-YYYYMMDD-HHMMSS-<row>.log`; those
live inside `qa-logs-1.zip`.

## What GrokBot delivered (assessment)

- **Coverage:** the complete `v0.3.1-minimal-release-test.md` (47 rows), the
  non-overlapping rows of `v0.3.1-total-test.md`, the Settings/YouTube extras from
  its own `gui-test-plan.md`, and the §6 Linux rows from the brief. Final count:
  83 PASS / 51 FAIL / 30 PARTIAL / 17 BLOCKED / 16 SKIPPED / 1 NOT RUN.
- **Quality:** high. Every FAIL has repro steps, expected/actual, screenshot and
  log names, and for the two crash classes a gdb stack plus a correct source
  pointer (ISS-1 `arrow_nav_sel_key` re-entrant egui lock; ISS-2 missing
  `gtk::init`). Duplicates were merged, environment-caused rows were marked
  BLOCKED/PARTIAL rather than FAIL, and it flagged its own synthetic-input
  limitations (ISS-7, ISS-18, ISS-62 need a real keyboard).
- **Gaps (by design of the run):** Linux X11 only, no Windows/macOS, no real
  display or GPU, no mouse drag-select, Alt-chords unreliable, no `Alt+click`
  (xfwm4). Nothing was tested after the fixes landed.

## Where the fixes are

| Source of fixes | Commit(s) | Covers |
|-----------------|-----------|--------|
| Round 1 bucket A/B (direct) | `5325e9b`, `97347d3` | ISS-1/3/4/8/10/11/17/20/27, ISS-2/5 |
| Round 1 orchestrator (buckets C/D/E) | `51f5a3a` | ISS-6/7/9/12/14/15/16/18/19/21/22/23/24/25/26/28/29 |
| Round 2 direct fixes | `5eec844` | ISS-35/37/38/43/45/46/53/55/57/58/62/63 |
| Round 2 orchestrator (tasks 1–33) | **uncommitted working tree** on top of `e68f833` | ISS-30/32/33/34/36/39/40/41/42/44/47/48/49/50/51/52/54/56/59/60/61 + SI-3/4, GIT-3, CSV-4, SPL-3, HTML-4 |

## Next steps

1. Commit the round 2 working tree (all 34 cyclopsctl tasks are `done`).
2. Human pass on Windows: `docs/technical/platform/v0.3.1-quick-manual-test.md`.
3. Hand `New/rerun-v0.3.1-round2.md` + current HEAD to GrokBot for the Linux re-run.
4. Any new FAIL becomes a new issue, not an edit of the round 2 PRD.
