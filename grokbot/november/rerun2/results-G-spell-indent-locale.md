# Ferrite v0.3.1 Linux — Rerun G (spell / indent / locale / misc)

Build tested: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (`9effd76`).

| ROW-ID | ISS | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|---|
| SPL-2 | ISS-47 | PASS | Spellcheck enabled via Settings; red squiggles under "speling"/"mistaek" in prose. Needed a clean user-words.txt (stale "mistaek" entry had suppressed it; emptied for the test). Setting only persisted after graceful exit (Alt+F4), not after pkill. | FAIL | `logs/ferrite-debug-20261109-132614-R2-SPL-2.log`, `logs/ferrite-debug-20261109-132656-R2-SPL-2b.log` | — |
| SPL-5 | ISS-47 | PASS | No squiggles in fenced code, inline code, URLs, camelCase, ALLCAPS or alnum tokens. | FAIL | same as SPL-2 | — |
| SPL-7 | ISS-48 | PASS | Wrap ON: squiggles appear near the viewport and sit under the right words on wrapped lines. | FAIL | same as SPL-2 | — |
| IND-2 | ISS-51 | PASS | Shift+Tab outdents a line with a leading hard tab (focus confirmed). | FAIL | `logs/ferrite-debug-20261109-132835-R2-IND-2.log` | — |
| TBL-3 | ISS-61 | PASS | Wrap OFF: raw table guides line up with body pipes. Header row differs slightly only where cells are unpadded in the source. Alt+Z skipped as instructed. | FAIL | `logs/ferrite-debug-20261109-132923-R2-TBL-3.log`, `logs/ferrite-debug-20261109-132951-R2-TBL-3b.log` | — |
| EXTRA-SET-7 | ISS-56 | PARTIAL | All 10 locales listed. In ja and zh-Hans, some Keyboard-category labels and group headers are still English, so the expectation "ja/zh keyboard labels translated" is only partly met. | FAIL | `logs/ferrite-debug-20261109-133100-R2-EXTRA-SET-7.log` | screenshot not saved to a file during this run (taken live only) |
| T2-32 | ISS-30 | PASS (documented) | Emoji render monochrome in Raw; some ZWJ/flag sequences blank or shown as letters. Documented limitation, not counted as FAIL. | FAIL | `logs/ferrite-debug-20261109-133236-R2-T2-32.log` | — |
| EMO-1 | ISS-30 | PASS (documented) | Same as T2-32 in Raw: monochrome emoji, some blanks. | FAIL | same as T2-32 | — |
| EMO-2 | ISS-30 | PASS (documented) | Rendered view: monochrome glyphs, some blanks/ZWJ breakage. Documented. | FAIL | same as T2-32 | — |
| TTL-6a | ISS-54 | PARTIAL | Double-click on title bar toggles maximize/restore. Drag-to-restore from maximized could not be shown to work; likely a limit of the synthetic drag, so not recorded as a product FAIL. | FAIL | `logs/ferrite-debug-20261109-133348-R2-TTL-6a.log` | — |
| STAT-3 | ISS-49 | PASS (by design) | Enter after a list item does NOT auto-continue the marker; matches documented behaviour. | FAIL | `logs/ferrite-debug-20261109-133509-R2-STAT-3.log` | — |
| EOL-9 | ISS-50 | PASS (by design) | Mixed file (CRLF, CRLF, CRLF, LF, then last line) edited with "Z" and saved: od -c shows per-line endings preserved, the LF line stayed LF and the others CRLF; the final unterminated line was written with CRLF. Check output: `logs/R2-EOL-9-check.txt`. | FAIL | `logs/ferrite-debug-20261109-133656-R2-EOL-9.log` | — |
| T3-43 | ISS-32 | DEFERRED (0.3.2) | No crash, all Mermaid blocks render. test_flowcharts.md "Subgraph Test (TD)": Node2→Node3 arrow runs through the "Inner Group" title; Inner Group drawn as a sibling box, not nested. "Nested Subgraph (Sibling)": First/Second side by side, A/B stacked, vs expected TB/LR. git graphs GG-01/02 render with ellipsised labels. | FAIL | `logs/ferrite-debug-20261109-133805-R2-MMD-10.log` | `rerun2/shots-G/R2-T3-43-subgraph-td-deferred.webp` |
| MMD-13 | ISS-59 | PASS | FC-165a CJK flowchart: node boxes hug the labels with small padding (no ~65% excess); layout identical on first paint and after scrolling away and back. Judged by eye, no pixel measurement. | FAIL | same as MMD-10 | `rerun2/shots-G/R2-MMD-13-cjk-flowchart.webp` |
| MMD-10 | ISS-60 | PARTIAL | FC-165b: badges 1–10 in source order, alt/else counted, labels and aliases shown. `autonumber off` and `autonumber 10 10` variants SKIPPED: not present in fixture and I did not edit diagrams. Right edge of the alt frame is clipped in the inline view. | FAIL | `logs/ferrite-debug-20261109-133805-R2-MMD-10.log` | `rerun2/shots-G/R2-MMD-10-autonumber.webp` |

## Extra observation (not a row)
Entering Rendered view on `test_git_graphs.md` (repo file) and on a copy of `test_mermaid_issue_165.md` marked the tab dirty (`*`) without any edit (165 char count changed 6376→6374). Same pattern as R2-HTML-6 spurious dirty. Closed with Don't Save. Screenshot: `rerun2/shots-G/R2-dirty-on-view-git-graphs.webp`.
