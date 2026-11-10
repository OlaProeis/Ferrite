# Ferrite v0.3.1 QA round 4 — SES-3d confirmation + Section B (MMD-3, MMD-3b)
Build: release binary at HEAD 1ad150b. Tester only: no code changes, commits or issues. Log: logs/ferrite-debug-*R4-3d-ON.log (one launch covered all rows; a 2nd launch only forwarded a file path to it: logs/*R4-B-fwd.log).

| Row | Result | Observed | Log lines | Evidence |
|---|---|---|---|---|
| 3d-confirm-ON | **FAIL** (cause: BLOCKED-by-environment for the file dialog, see notes) | Quick note ON. Ctrl+Shift+N, `abc`, secondary x -> "Unsaved changes in 'Untitled'. Close this window anyway?" Save, one deliberate click: dialog stays at +0/1/3 s, no new window (xdotool/wmctrl list identical before/after, only the 2 Ferrite windows + panel), nothing behind the Ferrite window. No Save As ever appears; window does not close; no file written. | `rfd::backend::xdg_desktop_portal ERROR pick_folder error No such file or directory (os error 2)`; `ferrite::files::dialogs Save dialog returned None on unknown (portal-requiring desktop). If no dialog appeared, check xdg-desktop-portal installation.`; `ferrite::app::file_ops Save dialog cancelled` | shots/C-3d-confirm-ON-{before,after1s,after3s}.png |
| 3d-confirm-OFF | **FAIL** (same cause) | Quick note OFF (Settings > Files, config verified): identical. Same dialog text, Save does nothing visible, dialog stays, no window. Not setting-dependent. | same 3 lines at 23:33:14Z | shots/C-3d-confirm-OFF-{before,after1s,after3s}.png |
| 3d-control (tab x, primary window) | no Save As (environment) | New Untitled tab, `abc`, tab x -> "'Untitled*' has unsaved changes. Close anyway?"; Save -> dialog closes, NO Save As window, tab stays open with `abc` (treated as cancelled). So the file dialog does not work under this Xvfb at all, independent of the window-close path. | same 3 lines at 23:32:27Z | shots/C-3d-control-after.png |

## Notes on 3d
- Root cause visible in the log: the file dialog backend (rfd) goes through xdg-desktop-portal, which is not installed/running on this box (`which xdg-desktop-portal zenity kdialog` -> none). So Save As cannot be driven here: environment limit, not proof the SES-3 close logic is right or wrong.
- Observable product difference, though: after a failed/cancelled Save As the TAB dialog closes (tab kept), whereas the WINDOW-close dialog stays open (log says "Save dialog cancelled" in both). A cancelled Save As arguably should leave the window-close prompt in a defined state; the dialog staying with no feedback looks like a UX gap. Whether the stay is intended is for the owner to judge. Recorded as observation; not filed.
- The file SES-3d-abc.md could not be produced: files/SES-3d-abc.md absent.
- Previous batch's "Return inserted a newline" was not retried (not re-asked).
- Quick note restored to ON (config.json verified ON at the end).

## Section B (Rendered view, 200 % = Ctrl+= x10 from 100 %, egui step 0.1; padlock unlocked (icon open); work copy work/B/test_mermaid_v031.md, md5 equals original; edited in Raw, switched to Rendered; never saved)
| Row | Result | Observed | Log lines | Evidence |
|---|---|---|---|---|
| MMD-3 | **PASS** | With `linkStyle 0,1,2,3 interpolate basis`: all four edges are S-curves (A->B, A->C, B->D, C->D), leaving/entering nodes horizontally. After deleting that line (line left empty) the same edges are straight diagonals. Edge A->B crops side by side show the curve vs a straight diagonal over its entire length (different path everywhere, not just the arrowhead). | no errors/panics | shots/B-MMD3-with-linkStyle-200.png, B-MMD3-without-linkStyle-200.png, B-MMD3-edgeAB-sidebyside.png (left with, right without), B-MMD3-diff.png |
| MMD-3b | **PASS** | Edges replaced by `A --> B --> C --> D` + `linkStyle 0,1,2 interpolate basis`: straight horizontal chain A-B-C-D. Without linkStyle: identical (pixel diff of the diagram area = only 24 px along the bottom border, edges identical). | no errors/panics | shots/B-MMD3b-with-linkStyle-200.png, B-MMD3b-without-linkStyle-200.png, B-MMD3b-diagram-stacked-with-over-without.png |

Notes B:
- The diagram header "Source >" toggle in Rendered view showed a truncated source (only `flowchart LR` / `A --> B`) at 200 % when expanded; not used for editing, and not judged against any row (observation only).
- During MMD-3b I mistyped once (UI repaints lag several frames; typing interleaved with clicks) and repaired lines 41-42 in Raw before judging; final source was verified in Raw before each rendered shot. Copy discarded via Don't Save; work copy md5 unchanged.
- No crash, no panic (grep panic|SIGABRT = 0). Ferrite stopped with SIGTERM at the end (exit 143 = my kill, not a crash).

## Config restore
config.json and session.json restored from config-backup-B/ and cmp-confirmed identical. quick_note_workflow = true. No Ferrite running.
