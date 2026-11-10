# Round 4 – Section D sanity rows (HEAD 1ad150b, release build, Xvfb 1920x1200)
cargo test row skipped per instructions (already done: 2249 passed).
Fresh fixture copies in work/D-<row>/. Padlock crops: shots/D-<row>-padlock.png (all four: OPEN shackle = UNLOCKED, bottom-right of the document pane).

| Row | Result | Observed | Log lines | Diff | Evidence |
|---|---|---|---|---|---|
| T1-18 | PASS | Rendered, UNLOCKED. Clicked the 2nd row of the paragraph before the blockquote (status Ln 30 Col 26, i.e. the line 29-30 paragraph). One ArrowDown: log `Some(29) -> line 32`; stopped there. Typed x; saved: x is inside the blockquote line; status Ln 32 Col 4. | files/D-T1-18-arrow-nav-grep.txt: `Rendered arrow nav Down: line Some(29) -> line 32` (1 line) | line 32 only: `> A quoted line...` -> `> xA quoted line...`; line 29 unchanged | shots/D-T1-18-{click,down1,afterx,saved,padlock}.png; files/D-T1-18.diff, D-T1-18-saved.md; logs/D-T1-18.log |
| ARR-4 | PASS | Split, UNLOCKED. Clicked preview paragraph "Another paragraph..." (Sec 2). Status bar `Ln 1, Col 1` before click, and still `Ln 1, Col 1` after ArrowDown x3 (400 ms apart). Preview caret moved (view scrolled to Sec 3-5 area); Raw pane caret/Ln unchanged. | files/D-ARR-4-arrow-nav-grep.txt: `Some(18) -> line 21`, `Some(21) -> line 23` (2 lines for 3 presses; the first press moved within the wrapped paragraph) | none (not saved, no edit) | shots/D-ARR-4-{before,after}.png, -{before,after}-status.png, -padlock.png; logs/D-ARR-4.log |
| T1-19 | PASS | Split, UNLOCKED. Python block Edit -> clicked end of `    return value` (caret visible) -> typed x (block shows `return valuex`, title still clean until commit) -> clicked outside block to commit (title `*`, 2013 chars, Raw pane line 42 reads `    return valuex`; no other line changed). One Ctrl+Z (log `Undo performed, 0 entries remaining`): back to 2012 chars, no `*`, Raw line 42 `    return value`, status Ln 42 Col 17. Then Ctrl+S; cmp vs original: byte-identical. | `Ctrl+Z (Undo) - consumed before render`; `Undo performed, 0 entries remaining`; `Saved file` (after undo) | empty diff (cmp IDENTICAL) | shots/D-T1-19-afterx-split.png (block only), D-T1-19-afterx-raw-line42.png (Raw pane line 42 with x), D-T1-19-afterundo*.png, -padlock.png; files/D-T1-19.diff (empty), D-T1-19-saved.md; logs/D-T1-19.log |
| ISS-68 | PASS | Clean test_github_html.md opened in Raw (title no `*`), switched to Rendered (V), waited >5 s (about 12 s), switched back to Raw: title `test_github_html.md - Ferrite`, no `*` at any point; file unchanged on disk. | `grep -c "Content modified in rendered editor"` = 0; no title with `*` set_title in the log | none (cmp unchanged) | shots/D-ISS-68-{0-raw,1-rendered,2-rendered-after5s,3-raw-final,padlock}.png; logs/D-ISS-68.log |

## Notes
- No crashes; all four Ferrite runs exited 0 (quit with Alt+F4). No Ferrite running now.
- Harness observations (not app bugs): the first mouse click after launch/window focus sometimes does not register and the screen repaints several seconds late; I re-screenshotted before acting. The first scrot copy of D-T1-18-saved.png was taken before the repaint; a later one is kept as shots/D-T1-18-saved_000.png.
- T1-18: the click landed on the paragraph's last visual row, so one press crossed to the blockquote (Some(29) -> line 32).
- T1-19: the Raw pane in Split does not show typed text until the block edit is committed (click outside); evidence taken after the commit and before Ctrl+Z, no save between x and undo. Ctrl+Z was pressed with the focus on the app window (not in the code editor).
- Ctrl+S (save) was only used in T1-18 (after the x) and T1-19 (after the undo).

## Config restore
Backup: config-backup-D/ (cp -a before start). At end, config.json and session.json copied back from it; `cmp` confirmed identical for both.
