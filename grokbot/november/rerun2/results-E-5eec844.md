# Ferrite v0.3.1 Linux — Rerun E (5eec844 direct fixes)

Build tested: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (`9effd76`).

| ROW-ID | ISS | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|---|
| T3-41 | ISS-35 | PASS | In Rendered `task_50_table_inline_formatting.md`, the cell shows bold, underlined “bold link”; no literal `**` markers. | PARTIAL: supplementary fixture previously showed literal `**` in link text. | `logs/ferrite-debug-20261109-125315-R2-T3-41.log` | — |
| TTL-6b | ISS-37 | PASS | With Use system title bar ON, Ctrl+, opened Settings. | PARTIAL: Ctrl+, was logged but did not open Settings. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | — |
| EXTRA-SET-1 | ISS-37 | PASS | With system title bar ON, the Settings gear also opened Settings. | FAIL: Settings entry path unavailable with system title bar ON. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | — |
| SES-1 | ISS-38, ISS-63 | PASS | Exit → Save dialog named `test_code_execution.md`, `crlf-test.md`, and `reload-test.md`; it said the modified Untitled note was not listed because Quick note workflow keeps it for the next launch. Save wrote all three named files and exited cleanly (code 0); the note returned after relaunch. | FAIL: saved files were saved but the Untitled tab was silently skipped, with no explanation. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log`; `logs/ferrite-debug-20261109-130614-R2-SES-5.log` | `shots/R2-SES-1-pass.png` |
| SES-5 | ISS-38 | PASS | The Untitled quick note text remained in the restored Untitled tab after relaunch. | PASS: prior run restored the dirty quick note from recovery. | `logs/ferrite-debug-20261109-130614-R2-SES-5.log` | — |
| T3-46 | ISS-38 | PASS | Exit dialog explicitly explained the Untitled/Quick note behavior; clicking the outline had not prevented the explanation. | Prior note: Untitled-only exit did not prompt; recovery retained text. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | — |
| EXTRA-SET-5 | ISS-43 | PASS | With Terminal Auto-load Layout OFF, opening the terminal panel did not load `.ferrite/terminal-layout.json`. | PARTIAL/FAIL: OFF persisted but the workspace layout still loaded. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | `shots/R2-EXTRA-SET-5-pass.png` |
| PRI-3 | ISS-45 | PASS | Middle-click on a tab closed that tab. | FAIL: repeated middle-clicks left tabs open. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | `shots/R2-PRI-3-pass.png` |
| UX-11 | ISS-53 | PASS | Ribbon MD quick reference included Reload (Ctrl+Shift+R) and word wrap (Alt+Z). | PARTIAL: F1 help listed them, but the MD reference did not. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | `shots/R2-UX-11-pass.png` |
| RUN-1 | ISS-55 | PASS | Python stdout printed before Stop remained visible. | PARTIAL: previous Stop/timeout outputs contained no stdout/stderr. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | `shots/R2-RUN-1-pass.png` |
| MMD-3 | ISS-57 | FAIL | Indexed `linkStyle 0,1,2 interpolate basis` still drew straight segments rather than curved edges. | FAIL: indexed linkStyle form had no effect. | `logs/ferrite-debug-20261109-115441-R2-MMD-1.log`; `logs/R2-MMD-3-log-tail.txt` | `shots/R2-MMD-3-fail.png` |
| MMD-6 | ISS-58 | PASS | Unknown `@pos` id produced a warning and the diagram rendered instead of falling back to source text. | FAIL: warning appeared but diagram fell back to raw source. | `logs/ferrite-debug-20261109-125315-R2-T3-41.log` | `shots/R2-MMD-6-pass.png` |
| TBL-3 (Alt+Z only) | ISS-62 | PASS | With editor text focused, real Alt+Z toggled word wrap without inserting a literal `z`; no other TBL-3 behavior was retested. | PARTIAL: prior TBL-3 Alt+Z typed `z` as a side effect. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | — |
| T2-39 | ISS-62 | PASS | Real Alt+Z toggled word wrap without inserting `z`. | FAIL: prior synthetic Alt+Z attempts inserted `z` without toggling. | `logs/ferrite-debug-20261109-125725-R2-TTL-6b.log` | — |
| SES-3 | ISS-63 | BLOCKED-by-environment | The second window contained only a modified Untitled quick note. In the follow-up, its close button and Alt+F4 left the window open without a prompt; no named saved-file tab was available to verify a file-naming dialog. Did not use a workaround to route a file into the second window. | PASS for save/close, but prior dialog omitted the affected filename. | `logs/ferrite-debug-20261109-130614-R2-SES-5.log` | `shots/R2-SES-3-blocked.png`; `shots/R2-SES-3-no-dialog.png` |
| SES-6 | ISS-63 | SKIPPED (deferred to later batch, as requested) | Not run. | Not run. | — | — |

## FAIL log tail

See `logs/R2-MMD-3-log-tail.txt`. The failure was visual; no linkStyle-specific diagnostic was emitted. The full process log exited cleanly (code 0).

## Cleanup

No live Ferrite process remained. The final UI-close attempt did not respond, so cleanup used SIGTERM (launcher recorded exit 143; this was a controlled cleanup, not a crash). Restored `~/.config/ferrite` from `rerun2/config-backup-E/` and restored modified test fixtures. `git -C /workspace/ferrite-qa/ferrite status --short`: clean.
