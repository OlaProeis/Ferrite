# Ferrite v0.3.1 Linux — Rerun F (rendered-edit cluster)

Build tested: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (`9effd76`).

| ROW-ID | ISS | RESULT | observation | prior | log | screenshot |
|---|---|---|---|---|---|---|
| TBL-2 | ISS-41 | PASS | Align cycling + cell edits in Rendered reached content; dirty `*` on first key; saved file delimiters/edits round-trip. | FAIL | `logs/*R2-TBL-2*.log` (if present) | — |
| EOL-3 | ISS-41 | PASS | Typed "Z" in Split; `*` on first key; saved file kept CRLF (eol-check/od). | FAIL | logs/ | — |
| EOL-5 | ISS-41 | PASS | Typed "Q" in Rendered; `*` on first key; saved file kept LF. | FAIL | logs/ | — |
| UND-3 | ISS-40 | FAIL | Typing in a Python code block in Rendered: only first char kept, rest dropped (retried slower, refocused). Undo then left file corrupted (stray "a" appended, heading displaced). See `rerun2/R2-UND-3-diff.txt`, `R2-UND-3-corrupted-file.md`. | FAIL | `logs/R2-UND-3-log-tail.txt` | see log tail / diff |
| CPY-2 | ISS-39 | PASS | Ctrl+C on selected Rust code block (first attempt empty clipboard, second after confirmed selection worked); clipboard byte-identical to source (od -c). | FAIL | logs/ | — |
| UX-8 | ISS-52 | PASS | Clicking in the middle of "tail" on mixed bold/italic/code/link line put caret inside the word (`ta#il`). Clicking its left edge landed one char early (before the space) — edge boundary note, not counted as fail. | FAIL | logs/ | — |
| UX-9 | ISS-42 | PARTIAL | Ctrl+click "CSV data file" in list item (test_md/test_local_links.md, Rendered): log `Opened link target via modifier-click: /workspace/ferrite-qa/ferrite/test_md/test_data.csv`. What opened: Chrome with directory index `file:///workspace/ferrite-qa/ferrite/test_md/` (parent folder; no file manager on box) — consistent with "files reveal parent", not the file itself. Chrome closed afterwards. Plain click/middle-click not exercised. | FAIL | `logs/ferrite-debug-20261109-132424-R2-UX-9.log` | `rerun2/R2-UX-9-chrome-dir-index.webp` |
| T3-44 | ISS-33 | PARTIAL | Adjacent `<div align>` blocks show their own text ("First centered block." / "Second block, right-aligned.") — ISS-33 text fix OK. But alignment not applied to those two (both left), and see HTML-6 spurious edit. | FAIL | `logs/ferrite-debug-20261109-132337-R2-HTML-6.log` | `rerun2/R2-HTML-6-spurious-dirty.webp` |
| HTML-6 | ISS-33 | FAIL | Merely switching a freshly opened clean `test_github_html.md` (pristine copy) Raw→Rendered, with no input, marks it dirty (`*`) and rewrites line 1 to `A# GitHub HTML Subset Test` (heading no longer renders). Reproduced twice (log: 3x "Content modified in rendered editor" at the mode switch). Disk file unchanged (not saved). | FAIL | `logs/ferrite-debug-20261109-132337-R2-HTML-6.log` (lines ~3055-3131) | `rerun2/R2-HTML-6-spurious-dirty.webp` |

## Log tails (FAIL rows)
- UND-3: `logs/R2-UND-3-log-tail.txt`
- HTML-6: `grep -n "modified\|view mode" logs/ferrite-debug-20261109-132337-R2-HTML-6.log` →
  `Title bar: Set view mode to Rendered` then 3x `Content modified in rendered editor, recorded for undo` (no user input).

Cleanup: Ferrite closed (no process), ~/.config/ferrite restored from config-backup-F, git status clean.
