## 7. Report format

Write `/workspace/ferrite-qa/qa-report-v0.3.1-linux.md` with these sections, then
**attach it and the `shots/` + `logs/` folders to your reply** (zip them if needed).

```markdown
# Ferrite v0.3.1 QA report — Linux

- Commit tested: <hash> (`0.3.1-experimental`)
- Date / timezone:
- Distro, kernel, display server (X11/Wayland), GPU/software GL:
- rustc version:
- Build time (release):

## Automated gate
| Check | Result | Numbers |
| cargo build --release | PASS/FAIL | warnings: N |
| cargo test --locked   | PASS/FAIL | N passed / N failed / N ignored |
| cargo clippy          | PASS/FAIL | N warnings, N errors |

## Results — minimal release test
| Row | Result | Evidence | Notes |
|-----|--------|----------|-------|
| 1 | PASS | shots/T1-1-pass.png | |
| 4b | FAIL | shots/T1-4b-fail.png, logs/run-1.log | lone 0a00 at offset … |
| … | SKIPPED (platform) | | |

## Results — total test (rows not already covered)
(same table, use the checklist IDs: EOL-7, MMD-13, HTML-2 …)

## Results — §6 extra rows
(same table: SIF-1, SI-1, PRI-1, SPL-1, FNT-1, IMG-X1 …)

## Issues found
| ID | Rows | Severity | Title | Exact repro steps | Expected | Actual | Evidence |
|----|------|----------|-------|-------------------|----------|--------|----------|
| ISS-1 | | blocker/major/minor/polish | | 1. … 2. … | | | shots/…, logs/… + last 30 log lines |

## Not tested and why
- Rows skipped for platform / network / hardware / blocked, each with the reason.

## Tag-gate opinion
One paragraph: based on the minimal test's "Tag gate (minimal)" section, is Tier 1
fully green on Linux? What would you fix before tagging?
```

Severity guide: `blocker` = crash, data loss, file corruption, cannot save/exit;
`major` = feature does not work as described; `minor` = wrong but workable;
`polish` = cosmetic.

For every `FAIL`, the Issues table **must** contain exact repro steps someone on
Windows can follow, plus the last ~30 lines of the debug log around the failure.
