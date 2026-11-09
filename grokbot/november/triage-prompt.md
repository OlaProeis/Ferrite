# Prompt: triage GrokBot re-run 2 (Nov 9) and propose the v0.3.1 round-3 close-out

@ai-context.md @docs/index.md @grokbot/README.md @grokbot/november/qa-rerun2-report-v0.3.1-linux.md @grokbot/New/rerun-v0.3.1-round2.md @docs/ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md

## Situation

Ferrite `0.3.1-experimental`, code at `221b6ae` (HEAD `9effd76`, pushed). All round-1 and round-2
GrokBot fixes are committed; `cargo test --locked` is green (Windows 2239, Linux 2228). I have **not**
done the Windows manual pass yet (`docs/technical/platform/v0.3.1-quick-manual-test.md`) — we are
dealing with the GrokBot results first.

GrokBot (autonomous tester, Linux X11 / Xvfb / xfwm4 / llvmpipe, synthetic xdotool input) re-ran the
gate in `grokbot/New/rerun-v0.3.1-round2.md` against `221b6ae`. Report:
`grokbot/november/qa-rerun2-report-v0.3.1-linux.md`. Evidence: `grokbot/november/rerun2-shots.zip`
(screenshots) and `rerun2-logs.zip` (debug logs, raw per-batch `rerun2/results-*.md`, gate logs).
Zips are git-ignored; unpack them under `grokbot/november/` (`shots/`, `logs/`, `rerun2/`) and read
the raw batch notes and logs where the summary row isn't enough.

Headline: **NOT ready to tag.** 98 rows → 65 PASS / 23 FAIL / 7 PARTIAL / 2 BLOCKED / 1 DEFERRED.
Tier 1: 20 PASS / 5 FAIL (T1-14, 17, 18, 19, 20). **No hang and no crash anywhere** — ISS-1 (22
freezes in October) and ISS-2 (GDK abort) are gone. 12 new issues filed: ISS-64 … ISS-75 (report §5),
plus PARTIALs/observations in §6 that were not filed.

## What I need from you

1. **Triage every new issue ISS-64…75 and every §6 PARTIAL/observation.** For each one:
   - Which earlier ISS / GitHub issue / fix commit / feature doc it relates to. Most are re-failures
     of rows that the `New/qa-report-v0.3.1-linux.md` Status column calls "fixed", e.g.
     ISS-64 ↔ ISS-1/#170 arrow nav (the deadlock fix may have disabled block navigation),
     ISS-65 ↔ ISS-6/ISS-40 code-block undo, ISS-66 ↔ ISS-5/ISS-46 X11 paste + toast,
     ISS-67 ↔ #177 multi-cursor Tab (T1-20), ISS-69 ↔ ISS-2 (now "Error 153", may be environment),
     ISS-71 ↔ IND-9, ISS-72 ↔ ISS-9 config toasts, ISS-73 ↔ ISS-57 linkStyle, ISS-74 ↔ SI-4 debounced
     save, ISS-75 ↔ SPL-3 tooltip guard.
   - **Why the earlier fix did not hold.** Read the code: is the fix Linux-gated or never reached on
     this path, is the unit test asserting the wrong thing, did a later round-2 task regress it, or is
     it an environment / synthetic-input artefact (read §7 harness limits before blaming the product)?
   - Severity (blocker = crash / data loss / corruption / cannot save; major; minor; polish) and a
     call: **fix for 0.3.1**, **defer to 0.3.2**, or **by design / environment**. ISS-65 (stray heading
     written into the document + broken undo) and ISS-68 (clean doc marked modified by switching to
     Rendered) are the two I consider most serious.
2. **Root-cause and propose concrete fixes** with file/function pointers (see `ai-context.md`
   "Where Things Live" and the feature docs in `docs/technical/`). Where a fix is small and isolated,
   implement it directly with a regression test; run only targeted `cargo test` filters. Where it is
   larger, write it up as a task.
3. **Write the round-3 close-out PRD** at `docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md`,
   same shape as the round-2 PRD (source, issue table with status, buckets: direct fixes vs orchestrator
   tasks, decisions table, ship gate, GrokBot re-run gate). Keep it parse-able by cyclopsctl
   (`cyclopsctl` conventions in `.cursor/rules/cyclopsctl/`). Then produce
   `grokbot/november/rerun-v0.3.1-round3.md` — the next GrokBot hand-off, same format as
   `grokbot/New/rerun-v0.3.1-round2.md`, listing only the rows that must be re-verified.
4. **Bookkeeping:** fill a Status column into the §5 issues of the November report (as the New report
   has), update `grokbot/README.md` (layout + "where the fixes are" + next steps) and `docs/index.md`
   (Release QA section) for the new files. Do not rewrite GrokBot's Actual/Expected text.
5. **Tell me what the Windows manual pass should focus on** given these results — update Part A of
   `docs/technical/platform/v0.3.1-quick-manual-test.md` if any of ISS-64…75 are platform-independent
   and need a real-keyboard check (ISS-64, 65, 67, 68, 70, 71 look platform-independent; ISS-66, 69,
   are Linux-only).

## Constraints

- Dev host is Windows; you cannot run the Linux GUI paths (X11 clipboard, WebKitGTK, xfwm4). Reason
  from code + logs, say what remains Linux-verify-only.
- Follow `ai-context.md` conventions: `anyhow::Result`, `log::`, explicit 0/1-indexing, line-ending
  helpers, no `unwrap` in library code. Document by feature in `docs/technical/`, add one bullet per
  new doc to `docs/index.md`. Don't edit `ai-context.md` rule sections; project-memory bullets only
  if you land code.
- Nothing from GrokBot's report is to be "fixed" by relaxing the test row; a FAIL stays a FAIL until
  code or an explicit by-design decision closes it.
- Finish with: (a) a ranked list of what blocks the tag, (b) what I should run manually today,
  (c) what goes to GrokBot next, (d) what we consciously defer to 0.3.2.
