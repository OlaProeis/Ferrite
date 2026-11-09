# Ferrite v0.3.1 — GrokBot re-run gate (round 3 close-out)

Hand this file to GrokBot on **Linux X11** (same environment as `grokbot/november/qa-rerun2-report-v0.3.1-linux.md`: Xvfb `:2` + xfwm4 + Mesa llvmpipe, `run-ferrite.sh`, debug logs under `logs/`). Install `scrot` and `xclip` **before** the first row (Batch A of round 2 lost its screenshots).

| Field | Value |
|-------|--------|
| **Branch** | `0.3.1-experimental` |
| **Build to test** | `0.3.1-experimental` HEAD, **`8c9c2d5` or later** (round-3 direct fixes, `prd-v0.3.1-qa-round3-closeout.md` §5.1; `git log --oneline` must include it). Rows marked *task N* additionally need that cyclopsctl task's commit — skip them (BLOCKED-by-build) if `git log` does not show it. |
| **Prior QA baseline** | `221b6ae` / HEAD `9effd76` — report `grokbot/november/qa-rerun2-report-v0.3.1-linux.md` |
| **Checklist** | `docs/technical/platform/v0.3.1-minimal-release-test.md` (Tier 1 gate) |
| **Log level** | `--log-level debug` for every row in the ISS-66 and ISS-64/65 tables (the fixes added `debug!` / `info!` / `warn!` lines that are part of the evidence) |

## Pass criteria

1. **Every row below** is **PASS** or **BLOCKED-by-input/environment/build** (say which). Any **FAIL** is filed as **ISS-76+** — do not edit the round-3 PRD or this file's Expected column.
2. **Tier 1** of the minimal checklist: all **PASS** or **BLOCKED-by-input/environment**.
3. For each FAIL attach `shots/` + `logs/ferrite-debug-*.log`; for ISS-64/65/68 rows also attach the **saved file** (`cp` after the test) so byte diffs can be checked.
4. Rows that passed in round 2 are **not** repeated here; only rows needing re-verification.

---

## A. Rendered-edit session (ISS-64, 65, 68) — direct fixes

Fixture copies into `/tmp/ferrite-rerun3/`. Run with `--log-level debug`.

| ISS | QA rows | Steps (unchanged from round 2 unless noted) | Expected at re-run | Evidence to grab |
|-----|---------|---------------------------------------------|--------------------|------------------|
| ISS-68 | HTML-6 repro, "dirty on view" (G/H) | Open clean copies of `test_md/test_github_html.md`, `test_md/test_mermaid_issue_165.md`, `test_md/test_git_graphs.md` in Raw; click **Rendered**; do not type or click in the document; wait 5 s; switch back to Raw; `wc -m` the file after **Don't Save** | No `*` in the title; **no** `Content modified in rendered editor` line; char count unchanged (6376 for the m165 copy); exit dialog does not ask to save. Note: line 1 of `test_github_html.md` is now `# GitHub HTML Subset Test` (fixture typo fixed) | log, screenshot of title bar |
| ISS-65 | T1-19, UND-1 | `test_copy_and_split_undo.md` → Split → click end of the Python block in the preview → type `x` → inspect Raw → Ctrl+Z once → Ctrl+Y → Ctrl+Z | Only line 43 gains `x`; line 49 stays blank (`diff` against the original shows exactly one changed line); **one** Ctrl+Z restores the original byte-for-byte; redo re-adds only `x`. Any `Skipping heading commit:` WARN in the log is itself a FAIL (means a stale block still existed) | log, `diff`, before/after screenshots |
| ISS-65 | UND-3 | Same fixture in **Rendered**; click into the Python block; type `abcde` at normal xdotool speed; Ctrl+Z | All five characters present before undo; one Ctrl+Z → file identical to original (`diff` empty) | log, `diff` |
| ISS-64 | T1-17 | `test_find_arrow_nav.md` → Rendered → click `## Section 1` → ArrowDown ×15 (≥ 100 ms apart) → type `X` → Raw | `X` is in the off-screen multi-line paragraph (Section 4), paragraph otherwise intact; heading unchanged | log, Raw screenshot |
| ISS-64 | T1-18 | Rendered; click the paragraph before the blockquote; ArrowDown; type `x` | `x` lands **inside** the blockquote line (`> …x`); preceding paragraph unchanged | Raw screenshot |
| ISS-64 | ARR-2 | Rendered; click the 2nd visual row of the wrapped Section 4 paragraph; ArrowDown | Caret moves to the next visual row / next block; status bar `Ln` changes | screenshot + status bar |
| ISS-64 | ARR-4 | Split; focus Rendered paragraph; ArrowDown ×3 | Caret walks blocks in the preview; Raw pane focus not stolen; Raw `Ln` unchanged | log |
| ISS-64 | T2-35 | Split + preview lock ON; ArrowDown in Rendered; type `Q` | Arrow moves the caret between blocks; `Q` **not** inserted (locked); Raw edits still work | screenshot |

## B. Raw editor / settings / popup (ISS-67, 72, 74, 75) — direct fixes

| ISS | QA rows | Steps | Expected at re-run |
|-----|---------|-------|--------------------|
| ISS-67 | T1-20 | `test_indent_selection.md` Raw; select Block 1 lines 7–9 (**verify the selection spans three lines** — in round 2 it was `(6,0)–(6,14)`, one line; if xdotool shift-drag cannot do it, use Shift+ArrowDown ×2 from line 7 col 0); Ctrl+click line 33 to add a caret; Tab; Ctrl+Z | Lines 7, 8, 9 each gain one indent step; **no text replaced**; line 33 gets the indent at the caret column (`中文行    首` is correct — the caret was mid-line); one Ctrl+Z restores the file byte-for-byte. Also run the single-line variant: select only `alpha line one` on line 7 → Tab → the whole line 7 is indented, text intact |
| ISS-72 | EXTRA-SET-9a | `{ not json` in `config.json`; launch; **screenshot within 20 s** of the window appearing | Toast "config was unreadable … renamed to …" visible; log has `Config load toast:` INFO line; file renamed `config.json.corrupt-<ts>` |
| ISS-72 | EXTRA-SET-9b | `"font_size": "abc"`; launch; screenshot within 20 s | Toast "1 invalid field reset"; `Config load toast:` INFO line; other settings kept |
| ISS-74 | SI-4 | Settings → Editor → untick **Show Line Numbers**; keep the mouse moving over the window for 2 s (app must be repainting); `pkill -9 ferrite`; inspect `config.json` | `"show_line_numbers": false` on disk, mtime within ~1 s of the click; log shows the settings save |
| ISS-74 | SI-4b | Same with **Spell check** toggle, wait 1 s, `pkill -9` | Persisted |
| ISS-75 | SPL-3 | Spell check on; hover `speling` until `[hint/spell] Unknown word` tooltip shows; right-click it; screenshot immediately and again after 500 ms | Tooltip gone on both screenshots; context menu's first suggestion row fully visible |

## C. Linux-only verification (ISS-66) — direct diagnostics, fix depends on your log

Run every step with `--log-level debug`. Report the **exact `Image paste:` lines** (or their absence) — they decide task 2 of the round-3 PRD.

| ISS | QA rows | Steps | Expected at re-run |
|-----|---------|-------|--------------------|
| ISS-66 | T1-14 | `xclip -selection clipboard -t image/png -i /tmp/red.png`; open saved `paste.md`; click end of a line (confirm caret visible); Ctrl+V (hold Ctrl ~0.4 s) | Either `![](assets/…png)` inserted + file under `assets/` **or** a failure toast. Log must contain one of: `Image paste: skipped, focus …`, `Image paste: release-only frame but clipboard holds text`, `Image paste: clipboard has no image`, `Clipboard image read failed`, or `Smart paste: Saved clipboard image`. **No line at all** is a FAIL with the note "release event not delivered" |
| ISS-66 | IMG-X1 | Same in a new **Untitled** tab | "save the document first" toast (or the same diagnostics) |
| ISS-66 | IMG-X2 | Ctrl+V twice within 1 s in the saved file | Two images or one image + no duplicate; no spam |
| ISS-66 | IMG-4, IMG-7 | Unblocked only if T1-14 passes | Per checklist |

If a real screenshot tool can be installed (`maim`, `gnome-screenshot`, `flameshot`), repeat T1-14 once with it as the clipboard owner and note the `TARGETS` list.

## D. Linux-gated — orchestrator task 1 (ISS-69); run only on the task-1 commit

| ISS | QA rows | Expected at re-run |
|-----|---------|--------------------|
| ISS-69 | T2-36, VID-1 | Trusted YouTube embed: thumbnail → click → player loads and **plays** (or the player's own "video unavailable" for a dead id) — **no Error 153**. Note `libwebkit2gtk-4.1-0` version |
| ISS-69 | EXTRA-YT-2 (offline, `unshare -n`) | Placeholder rect with URL + "Open in browser" — no black GTK surface; no toast storm |
| ISS-69 | EXTRA-YT-4 (12 embeds) | Scroll top→bottom ×3 responsive; **no** `Gdk-CRITICAL _gdk_frame_clock` lines |
| ISS-69 | EXTRA-YT-6, EXTRA-YT-7 | Live URL edit / width update; Reload and Search dialogs above players — re-check on a real keyboard if xdotool appends |

## E. Fixtures changed / by-design rows (confirm the new expected result)

| Row | ISS | What changed | Expected at re-run |
|-----|-----|--------------|--------------------|
| MMD-3 | ISS-73 | `test_md/test_mermaid_v031.md` §3 is now a fan-out/fan-in diagram (`A→B, A→C, B→D, C→D`, `linkStyle 0,1,2,3 interpolate basis`) with a note that a collinear chain cannot curve | Edges visibly curved and **different** from the same diagram without `linkStyle`; screenshot both |
| HTML-6 / T3-44 | ISS-33 | Fixture title typo fixed (`# GitHub HTML Subset Test`) | Heading renders as H1; adjacent `<div align>` still show own text. **T3-44 alignment (centre/right) stays a known PARTIAL until task 4 — not a new FAIL** |
| IND-9 | ISS-71 | **Deferred to 0.3.2** — rendered list Tab is not implemented | Record "Tab in a rendered list item does nothing; app responsive; Raw unchanged" → **BLOCKED-by-decision**, referencing PRD §8. Do not mark PASS |
| CPY-3 | ISS-70 | **By design 0.3.1** — clicking a formatted paragraph edits its source | Record clipboard = source markdown → **BLOCKED-by-decision** (PRD §8). Do not mark PASS |

## F. Observations from §6 to re-test (no code change unless stated)

| Row | Task | Steps | Expected |
|-----|------|-------|----------|
| SES-3 window | task 3 | Ctrl+Shift+N → in the new window Ctrl+N → type `abc` → × on that window; then Alt+F4 | Either the save prompt appears **in that window** or (Quick-note ON) the window closes; the window never silently refuses. Also test with Quick-note OFF |
| SI-3 | — | Make the primary dirty (type, don't save, wait for `recovery/*.json`); launch `--new-instance`; quit the secondary | Primary's `recovery/` files untouched; `session.json` not overwritten |
| SI-1 | — | Minimize the primary; forward a file open | Note whether the window is restored. If `_NET_WM_STATE_HIDDEN` stays, record BLOCKED-by-WM with the xprop output |
| TTL-6a | — | Real display if available | Drag-from-maximized restores |
| `cargo test` count | — | `cargo test --bin ferrite -- --list 2>/dev/null \| grep -c ': test'` | Linux count = Windows count − 11 (Windows on the round-3 fix commit: **2246 passed** → expect 2235 on Linux). The 11 `#[cfg(windows)]` tests are: `path_utils::tests::windows_tests::{test_normalize_verbatim_disk_path, test_normalize_verbatim_disk_root, test_normalize_regular_windows_path, test_normalize_unc_path, test_normalize_lowercase_drive, test_canonicalize_and_normalize, test_canonicalize_or_normalize_nonexistent}`, `path_utils::tests::{test_parse_windows_absolute_path, test_convert_wsl_mount_path_to_windows_path}`, `vcs::git::tests::{test_repo_relative_path_handles_windows_verbatim_prefix, test_git_service_untracked_file_with_canonicalized_path}`. Confirmed — close the §6 item |

---

## Suggested run order

1. **Section A** first (highest value, fresh build): ISS-68 dirty-on-view ×3 fixtures, then T1-19 / UND-1 / UND-3, then T1-17 / T1-18 / ARR-2 / ARR-4 / T2-35.
2. **Section B**: T1-20 (both variants), SI-4 ×2, SPL-3, EXTRA-SET-9a/b (config backup/restore as in round 2).
3. **Section C**: ISS-66 with debug log — report the exact lines even on PASS.
4. **Section E**: MMD-3 new fixture, HTML-6, IND-9 and CPY-3 as BLOCKED-by-decision.
5. **Section F**: SES-3, SI-3, SI-1, test count.
6. **Section D** last and only if `git log` shows the task-1 commit.

Tier 1 rows from `v0.3.1-minimal-release-test.md` not listed here passed in round 2 and are not required again unless a fix above touches them (T1-19, T1-20, T1-17, T1-18, T1-14 are the ones that do).

---

## Related docs

- Round 3 PRD: `docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md`
- Round 2 report (source of ISS-64…75): `grokbot/november/qa-rerun2-report-v0.3.1-linux.md`
- Round 2 re-run doc (superseded by this file): `grokbot/New/rerun-v0.3.1-round2.md`
- Fix docs: `docs/technical/markdown/rendered-session-scope-and-seed.md`, `docs/technical/editor/indent-selection.md`, `docs/technical/config/settings-debounced-save.md`, `docs/technical/config/config-load-toasts.md`, `docs/technical/editor/diagnostic-hover-popup-guard.md`
- Agent brief: `docs/technical/platform/v0.3.1-grok-bot-qa-brief.md`
