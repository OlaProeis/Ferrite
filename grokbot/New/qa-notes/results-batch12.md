# Ferrite v0.3.1 QA - Batch 12a (HTML/tables/CSV/UX/workspace remainder)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (ferrite 0.3.1, repo HEAD 8e6ef508, clean tree at start). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <files>`. Screenshots: `/workspace/ferrite-qa/shots/`. Logs: `/workspace/ferrite-qa/logs/`.
Started: Fri Oct 9 2026 ~04:18 CEST.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| HTML-4 | PASS | test_github_html.md Rendered (instance WS-1): `H<sub>2</sub>O`, `CO<sub>2</sub>` show lowered digits; `E=mc<sup>2</sup>`, `x<sup>n</sup>`, `word<sup>1</sup>`, `10<sup>−3</sup>` show raised digits; `reference<sub>note</sub>` lowered; `Line one<br>Line two<br>Line three` renders as three visual lines in one paragraph; standalone `<br>` block produces a blank line gap. Side note (not a row verdict): the heading `## Phase 1 — <br> line breaks` shows a box glyph (□) and heading `## Phase 2 — <kbd>` drops the `<kbd>` text. | logs/ferrite-debug-20261009-041820-WS-1.log (instance WS-1 also covers HTML-5, HTML-6, UX-4, UX-7, UX-11, WS-1, WS-2 and later rows; no panic) | shots/HTML-4-pass.png |
| HTML-5 | PASS | Same doc, `<img>` sizing: `width=120 height=40` renders at 120x40 physical px (screenshot 80x27 at 2/3 scale); `width=400 height=200` renders 400x200 (fits viewport); `alt`/`title` only renders at natural 64x64. Pixel check on shots/HTML-5-pass.png (display is 1920x1200, screenshots downscaled): orange bbox 79x33 for the 120x40 image (expected from native sample.png 64x64 scaled to 120x40: 78.75x32.5), 266x164 for 400x200 (expected 262.5x162.5 with AA edges), 42x52 for natural 64x64 (expected 42x52). | logs/ferrite-debug-20261009-041820-WS-1.log | shots/HTML-5-pass.png |
| HTML-6 | FAIL | Single-line `<div align>` blocks in test_github_html.md "Adjacent divs" section (lines 42 and 44) do NOT render their own text: `<div align="center">First centered block.</div>` (line 42) and `<div align="right">Second block, right-aligned.</div>` (line 44) BOTH display the doc's first line `A# GitHub HTML Subset Test` (left-aligned, no centering/right alignment; "First centered block." and "Second block, right-aligned." never appear). Line 46 regular paragraph after them is fine. Multi-line `<div align>` blocks (Left/Center/Right sections, lines 13-38) render correctly. Other single-line div in fixture: line 199 `<div onclick="alert(1)">Click me …</div>` (Unsafe section) renders as a bare «HTML» passthrough indicator with its text not shown (acceptable per fixture's "indicator" expectation; no script run). This matches the T3-44 symptom. | logs/ferrite-debug-20261009-041820-WS-1.log (no WARN/ERROR log line tied to this; see FAIL excerpt below) | shots/HTML-6-fail.png |
| WS-2 | PASS | Stats tab found in the right-side Document panel (tab strip Outline | Stats | Links | FM | Hub; click the "Stats" tab next to Outline; status bar also shows `LSP: Disabled`). Runtime Modules section lists: Loaded fonts = Emoji; Mermaid cache 0 / 0; Terminal sessions 0; Language Server = Disabled (grey). | logs/ferrite-debug-20261009-041820-WS-1.log (same instance) | shots/WS-2-pass.png |
| UX-4 | PASS | Outline panel (right panel, Outline tab; opened with Ctrl+Shift+O) on test_find_arrow_nav.md (8 headings; instance WS-1): indentation guide lines connect parents to children; bullets are coloured circles per element type (H1/H2 bold, H3 regular, Code block grey, Table orange, Blockquote/Markdown image markers) with collapse chevrons on parents; current-section PILL (light rounded background + blue title) sits on the heading that contains the caret: on open it is on the H1 (caret Ln 1), after clicking the "Section 3" outline item the caret moves to Ln 27 and the pill moves to Section 3 (minimap also highlights it). Hover tooltips present: "Heading 2 · Section 4 · line 44" and (test_github_html.md) "Heading 2 · Unsafe HTML (must stay passthrough / not execute) — line 191". Note: on test_github_html.md the caret sits at Ln 1 (before first heading), so no pill is shown there (expected). | logs/ferrite-debug-20261009-041820-WS-1.log | shots/UX-4-pass.png |
| UX-7 | PASS | Two tabs: switched A = test_github_html.md -> B = test_find_arrow_nav.md (title bar "test_find_arrow_nav.md - Ferrite"); clicked the ribbon back-to-previous-tab arrow (left of the search icon) -> active tab returned to test_github_html.md (title "test_github_html.md - Ferrite", tab highlighted). Sequence repeated once more (B -> back -> A) to take the evidence screenshot, same result. | logs/ferrite-debug-20261009-041820-WS-1.log | shots/UX-7-pass.png |
| UX-11 | PARTIAL | F1 opens the "About / Help" tab; its Shortcuts section lists `Ctrl+Shift+R` = "Reload from Disk" (File group) and `Alt+Z` = "Toggle Word Wrap" (View group) — F1 part PASS. The ribbon "MD" button (tooltip "Markdown quick reference") opens the "Markdown guide" popup, which lists only Bold (Ctrl+B), Italic (Ctrl+I), Inline code (Ctrl+Shift+`), Link (Ctrl+K), Code block (Ctrl+Shift+C), Math, Superscript, Regex escape, Footnote, Task list (scrolled to end) — it does NOT list Reload (Ctrl+Shift+R) or word wrap (Alt+Z). So the Markdown quick reference half of the expectation is not met. | logs/ferrite-debug-20261009-041820-WS-1.log | shots/UX-11-partial.png (MD popup, before close) |
| WS-1 | PASS | Launched with repo folder as workspace (`/workspace/ferrite-qa/ferrite`) plus two fixture tabs. No Session Recovery dialog (session.json clean_shutdown=true; 50 earlier tabs restored silently from the shared session file, not Start Fresh). File tree (left): hover highlight on rows (e.g. hovering `test_indent_selection.md` gives a grey row background, pixel ~ (227,227,232) vs normal (245,245,245), plus tooltip); the row of the active tab `test_github_html.md` (after expanding `test_md`) has emphasis: light-blue row background (220,234,247) and bold blue file name. Emphasis follows the active tab (seen on test_github_html.md and when test_find_arrow_nav.md was active). | logs/ferrite-debug-20261009-041820-WS-1.log | shots/WS-1-hover.png |
| CSV-5 | PARTIAL | Fixtures `test_md/test_data.csv` and `test_md/test_data.tsv` have NO long values (longest field 19 chars in CSV, 20 chars in TSV), so their grid views (CSV grid, TSV split/grid via Ctrl+E) simply fit every value in its column: nothing to clip. To exercise the clipping rule I made scratch copies OUTSIDE the repo at `/tmp/csv5/long_values.csv` and `/tmp/csv5/long_values.tsv` (same columns, values of ~190 and ~130 chars, opened in the running instance). Result on those copies (grid view): long `name` cell `Lorem ipsum dolor sit amet, consectetur adipisc…` is clipped with an ellipsis at the column border (next column `age` starts right after, no overflow); long `city` cell `A very long city name that must be clipped insi…` is clipped at the `occupation` column border; same in the TSV grid. No text runs into neighbouring columns; short values and numbers stay aligned. Verdict PARTIAL only because the requested fixtures contain no long values. | logs/ferrite-debug-20261009-041820-WS-1.log (copies opened by forwarded launch, log logs/ferrite-debug-20261009-042501-CSV-5.log is empty by design) | shots/CSV-5-csv-copy-pass.png, shots/CSV-5-tsv-copy-pass.png |
| CSV-4 | PASS | `test_md/very_large_test_data.csv` (85,669,226 bytes, 500,001 data rows) opened lazily in the running instance (forwarded launch): Raw view first, log `Large file detected (81MB), some features may be disabled`, process stayed responsive (no hang, RSS ~500 MB). Switched to the grid via the top-right R/S/V view-mode control (V = Rendered grid; Ctrl+E alone did nothing on this file in Raw). Grid shows header `Column_0..Column_14`, footer/status `500001 rows total (lazy-parsed)`. Edit attempt: double-click and single-click on cell `Name_0` (row 1, Column_1): no editor opened; the prompt shown is the banner text **"File is large (85.7 MB). Cell editing is disabled — switch to Raw view (Ctrl+E) to edit."** (persistent banner, not a modal; it appeared on opening the grid). Second banner (red): "File is large (85.7 MB). Showing raw content." with buttons Dismiss / Show Raw, while the grid is still visible (wording mismatch, cosmetic). File on disk unchanged (repo clean, mtime Oct 8 19:53). | logs/ferrite-debug-20261009-041820-WS-1.log (instance covers the open; forwarded launch log logs/ferrite-debug-20261009-042649-CSV-4.log is empty by design) | shots/CSV-4-pass.png |
| TBL-3 | PARTIAL | Raw view of `test_md/test_tables.md` and `test_md/alignment_test.md`. **No source mutation / no dirty marker from viewing: PASS** — tab titles had no `*` after opening and viewing; `git -C /workspace/ferrite-qa/ferrite diff --stat` empty, `git status --short` empty (checked several times). **Column guides: PARTIAL.** Ferrite draws display-only table guides in Raw only when word wrap is OFF (`src/editor/ferrite/table_guides.rs`: `if wrap_enabled { return; }`; the setting was `word_wrap: true`, so no guides are visible by default). With wrap OFF (Alt+Z, later restored) the guides are continuous vertical lines at the HEADER's pipe columns, but body-row pipes do not coincide with them in the proportional Raw font: in test_tables.md "Wide Table (5 columns)" the body pipes (e.g. after `A1`, `B1`) sit ~20 px left of the guides and the guides run through body text; in alignment_test.md the guide at the `Column 2` boundary runs through the `Cell 2` text of the body row. The Simple 2x2 and 3x3 tables show the same pattern. So guides line up with header pipes only. **Side effect observed (not a verdict row, see notes):** pressing Alt+Z while the Raw editor of alignment_test.md had focus toggled word wrap AND inserted a literal `z` at the start of line 1 (tab became `alignment_test.md*`); Ctrl+Z removed it (status "Undo (0 remaining)", no `*`), disk untouched. Word wrap restored to ON via Settings (Editor > Word Wrap checkbox), config.json `"word_wrap": true` again. | logs/ferrite-debug-20261009-041820-WS-1.log | shots/TBL-3-tables-raw.png (wrap on, no guides), shots/TBL-3-alignment-raw.png (wrap on), shots/TBL-3-tables-nowrap.png (wrap off, guides), shots/TBL-3-guides-nowrap.png (wrap off, alignment_test) |
| TBL-2 | PARTIAL | Copy `/tmp/tbl2.md` (repo fixture untouched) opened in the running WS-1 instance (forwarded launch). In Rendered, the table toolbar's Align buttons (three icons: left / center / right) apply to the column of the focused cell. Column 1 set to left, column 2 left to center, column 3 center then right. Delimiter row round-trip on disk after Ctrl+S (`sed -n 40,43p /tmp/tbl2.md`): `\|:----\|:----:\|----:\|` = left / center / right, i.e. `:--`, `:-:`, `--:` forms all round-trip correctly (PASS for alignment part). Rendered cell edit: clicked into the "More" cell (col 1, row 2) and typed `X`; it rendered as `MorXe` with the caret at the click point. PARTIAL reason: the in-progress cell edit was NOT in the source when I toggled to Raw and pressed Ctrl+S (Raw showed `More`, saved file had `More`); the `X` appeared in Raw and on disk only after I returned to Rendered and clicked into another cell (cell edit seems to commit on focus loss, not on each keystroke or on view toggle). Cosmetic: a cell in edit mode shows its text left-aligned even when its column is centred. Process stayed responsive, no panic. | logs/ferrite-debug-20261009-041820-WS-1.log (instance covers TBL-2; forwarded launch log logs/ferrite-debug-20261009-043332-TBL-2.log is empty by design: clean exit 0 after forwarding) | shots/TBL-2-pass.png (Raw delimiter after edits; earlier: shots/TBL-2-rendered-before.png, shots/TBL-2-after-edit.png, shots/TBL-2-raw-after.png) |
| UX-8 | FAIL | Plain-text and line-start clicks accurate; click on 'tail' after bold/italic/code/link markup put caret ~10 source chars right (after 'text '). Reproduced on two /tmp copies. | logs/ferrite-debug-20261009-041820-WS-1.log | shots/UX-8-clicked.png, shots/UX-8-fail.png |
| UX-9 | FAIL (hang) | Plain click on a file link switched the list item to source, then Ferrite deadlocked (main thread in arrow_nav_sel_key / render_session_formatted_edit_text rwlock); reproduced in a fresh instance. Ctrl+click / middle-click on file, folder, web links opened nothing (no thunar/xdg-open/browser process, no log line). Ignored schemes not tested. | logs/ferrite-debug-20261009-041820-WS-1.log, UX-9-fresh log, logs/UX-9-hang-bt.txt, logs/UX-9-fresh-2-hang-bt.txt | shots/UX-9-fail.png, shots/UX-9-fail-2.png |

## FAIL log excerpts (span-filtered, logs/ferrite-debug-20261009-041820-WS-1.log)

### HTML-6 (lines 597-627)
```
[2026-10-09T02:21:13Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:21:13Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:21:19Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:21:19Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:21:19Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:21:19Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:21:19Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:21:19Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:21:19Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:21:24Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:21:24Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:21:24Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:21:24Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:21:24Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:21:24Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:21:24Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:21:25Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:21:25Z DEBUG ferrite::state] Switched to strip tab 1 in window 0
[2026-10-09T02:21:25Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'vid-vimeo-only'
[2026-10-09T02:21:25Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:21:25Z DEBUG winit::window] winit::Window::set_title; title="vid-vimeo-only.md - Ferrite"
[2026-10-09T02:21:25Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:21:29Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:21:29Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=51, clean_shutdown=false
[2026-10-09T02:21:29Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:21:29Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:21:29Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 51 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:21:29Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:21:29Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:21:33Z DEBUG ferrite::state] Switched to strip tab 21 in window 0
[2026-10-09T02:21:33Z DEBUG ferrite::app::navigation] Backlinks (cached index): 5 backlinks for 'test_github_html'
```
### UX-8 (lines 13257-13286)
```
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:39:47Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:39:47Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] FerriteEditor max_line_width setting: Col100 -> Some(840.00006)px (char_width=8.4)
[2026-10-09T02:39:47Z DEBUG ferrite::editor::widget] Syncing 1 fold regions to FerriteEditor, show_indicators=false
[2026-10-09T02:39:47Z DEBUG ferrite::editor::ferrite::editor] Word wrap: text_area_width=1210.3, max_wrap_width=840.0, effective=840.0
[2026-10-09T02:39:47Z DEBUG ferrite::app::title_bar] Title bar: Set view mode to Rendered
[2026-10-09T02:39:47Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=false
[2026-10-09T02:39:52Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:39:52Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=58, clean_shutdown=false
[2026-10-09T02:39:52Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:39:52Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:39:52Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 58 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:39:52Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:39:52Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_cursor_visible; visible=true
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_ime_allowed; allowed=true
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 301 }) size=Physical(PhysicalSize { width: 818, height: 34 })
[2026-10-09T02:39:54Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 301 }) size=Physical(PhysicalSize { width: 818, height: 34 })
[2026-10-09T02:39:55Z DEBUG ferrite::app::keyboard] Keyboard shortcut: Ctrl+S (Save)
[2026-10-09T02:39:55Z INFO  ferrite::state] Saved file: /tmp/ux8/ux8b.md (encoding: UTF-8)
[2026-10-09T02:39:55Z DEBUG ferrite::app::file_ops] File saved successfully
[2026-10-09T02:39:55Z DEBUG ferrite::app] Cleaned up auto-save temp file for tab 109
[2026-10-09T02:39:55Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 519, y: 301 }) size=Physical(PhysicalSize { width: 818, height: 34 })
[2026-10-09T02:39:55Z DEBUG ferrite::app::navigation] Backlinks (cached index): 0 backlinks for 'ux8b'
```
### UX-9 (tail)
```
[2026-10-09T02:40:43Z DEBUG winit::window] winit::Window::set_ime_cursor_area; position=Physical(PhysicalPosition { x: 535, y: 475 }) size=Physical(PhysicalSize { width: 356, height: 17 })
[2026-10-09T02:40:43Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:43Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:43Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:48Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:48Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:48Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:53Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:53Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:53Z DEBUG ferrite::app] Crash recovery snapshot saved
[2026-10-09T02:40:58Z DEBUG ferrite::state] Capturing session state with workspace: /workspace/ferrite-qa/ferrite (canonical: /workspace/ferrite-qa/ferrite)
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Saving session state: is_recovery=true, app_mode=Workspace(/workspace/ferrite-qa/ferrite), windows=1, tabs=60, clean_shutdown=false
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Writing session to temp file: /home/box/.config/ferrite/session.recovery.tmp
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Renaming temp file to: /home/box/.config/ferrite/session.recovery.json
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Successfully saved session state to /home/box/.config/ferrite/session.recovery.json (1 windows, 60 tabs, Workspace(/workspace/ferrite-qa/ferrite))
[2026-10-09T02:40:58Z DEBUG ferrite::config::session] Saved recovery content for tab 89 (path=None, hash=None)
[2026-10-09T02:40:58Z DEBUG ferrite::app] Crash recovery snapshot saved
=== Ferrite exited with code 137 SIGKILL (e.g. kill -9) at 04:42:16 CEST ===
```

## Final state

pgrep -a -x ferrite: empty

```
git status --short:
```

## Batch 12b (code run / git / title bar / YouTube overlays / MW-5)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite` (ferrite 0.3.1). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> <file-or-folder>`. Screenshots: `/workspace/ferrite-qa/shots/`. Logs: `/workspace/ferrite-qa/logs/`.
Setup notes: RUN-1 needed the first-run consent modal, so `code_execution_consent_acknowledged` was temporarily set to `false` in `~/.config/ferrite/config.json` (Ferrite closed; original saved at `/tmp/batch12b/config.json.orig`; restored at the end). Timeout left at the default 30 s. Scratch copies live in `/tmp/batch12b/` (repo fixtures were not edited).

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| RUN-1 | PARTIAL | Instance RUN-1 on `test_md/test_code_execution.md` (Rendered). **Consent:** Run on §2c Python showed modal "Allow running code from this document?"; body "Running a fenced snippet starts a real subprocess on your computer using your user privileges. Ferrite does not sandbox runners or filter network access." plus "Same caveats as in Settings → Editor → Code execution: no sandbox — only use with documents you trust."; buttons Enable & run / Just enable / Cancel. **Python output: PASS**, after Enable & run the block showed `Hello from Python`, status "Exit 0 · 117ms", with Insert as block / Copy / Dismiss. §4 Python stderr block also ran (Exit 0, stdout + stderr section). **Timeout: PASS**, §10 `time.sleep(60)` showed "Timed out after 30s" at 30.01 s, output "(no stdout or stderr)" (default 30 s timeout used; the fixture's 5 s setting was not applied). **Stop: PASS**, §11 loop stopped by the Stop button at ~9.8 s, status "Stopped by user", Run re-enabled, no spinner. **PARTIAL reason:** the §11 fixture expects partial stdout (`Running…`/ticks) after Stop, but none was shown. `code_execution.rs` has no PYTHONUNBUFFERED, so piped Python prints are block-buffered and lost when the process is killed (same for §10 "Starting sleep…"). | logs/ferrite-debug-20261009-044740-RUN-1.log (instance log; no code-run lines at debug level) | shots/RUN-1-consent-pass.png |
| RUN-4 | PASS | Scratch doc `/tmp/batch12b/run4.md` (`time.sleep(12)` before the first print), Rendered, instance RUN-1 with the file forwarded. Clicking Run showed status "Running · 3.66s", Stop button and the placeholder **"Waiting for output…"** in the output panel. After 12.03 s the output `late output after 12 s` replaced the placeholder with "Exit 0 · 12.03s". The same placeholder was also seen on the repo fixture §10/§11 blocks while they ran with no output yet. | logs/ferrite-debug-20261009-045149-RUN-4.log (forwarded launch, near-empty by design); instance log logs/ferrite-debug-20261009-044740-RUN-1.log | shots/RUN-4-pass-sleep12.png, shots/RUN-4-pass.png, shots/RUN-4-after-output.png |
| RUN-5 | PASS | §4 Python block of `test_code_execution.md` (`print(... file=sys.stderr)`): output panel shows stdout lines, then a **"── stderr ──"** heading with `stderr line` under it, status "Exit 0 · 21ms". **Copy** (labelled "Copy", tooltip "Copy output to clipboard") works: `xclip` read back `stdout line` / `stdout again` / `── stderr ──` / `stderr line`, with no ANSI codes. **Insert as block** (labelled, tooltip "Insert the captured output as a fenced ```output block in the source") works on the scratch copy `/tmp/batch12b/run-stderr-copy.md`: an `output` block with the same stdout + "── stderr ──" + stderr text appeared directly below the §4 Python fence. The inserted text was not saved to disk (repo fixture untouched). | logs/ferrite-debug-20261009-045051-RUN-5-insert-copy.log (forwarded launch, near-empty by design); instance log logs/ferrite-debug-20261009-044740-RUN-1.log | shots/RUN-5-pass.png |
| GIT-2 | PASS | Launched with `/workspace/ferrite-qa/ferrite` as workspace (no Session Recovery dialog; the tree shows the workspace root with a git badge and branch `0.3.1-experimental` in the status bar). After the Shell step `echo hi > test_md/zz_untracked.md`, expanding `test_md` in the file tree showed `zz_untracked.md` with the untracked badge (green name plus `?`). No refocus was needed. `git status --short` gave `?? test_md/zz_untracked.md`. | logs/ferrite-debug-20261009-045325-GIT-2.log | shots/GIT-2-pass.png |
| GIT-3 | PARTIAL | Opened `test_md/zz_untracked.md` from the tree, edited, Ctrl+S: the tab and tree show the saved file, and the tree keeps `zz_untracked.md ?`. Edited tracked `test_md/wikilink-target-a.md` (Raw, appended `x`), Ctrl+S: the tree shows an orange **modified dot** on `wikilink-target-a.md`, matching `git status` (` M`). No `\\?\` prefix anywhere in the UI (tree, tabs, status bar) and 0 `\\?\` strings in the log. Cleanup: `git checkout -- test_md/wikilink-target-a.md` and `rm test_md/zz_untracked.md`; the tree dropped both badges and the file without a refocus. **PARTIAL reason:** the refocus step was not exercised: the window never lost focus (alt+Tab did not switch windows, and the title-bar click gave no focus event; the log only has "Window focus gained" at startup 02:53:26). **Observation (not a row fail):** each save of the new untracked file logged `Reloaded externally modified file` right after `Saved file` (2 times, log lines 02:54:28 and 02:54:54), while the tracked-file save did not. One save ran while my typing was still going, so the disk got `hi` / `edi` and the buffer was reloaded before the rest of the typed text. The second save wrote the full text. | logs/ferrite-debug-20261009-045325-GIT-2.log | shots/GIT-2-pass.png, shots/GIT-3-modified-before-refocus.png, shots/GIT-3-pass.png |
| TTL-6a | FAIL (minor) | Default custom title bar (`use_system_title_bar` false), instance GIT-2, measured with `xdotool getwindowgeometry` (read-only) and screenshots. **Double-click maximized → restored: the title-bar icon flipped, but the window stayed 1920x1200 at x=-232 (left part, file tree and tab start off-screen), so it is not a normal-size restore.** A second double-click on the restored window did not maximize (no change). The maximize button did maximize (0,0, 1920x1200). **Drag window (restored): PASS**, a slow multi-point drag from the title bar moved the window about +120/+85 px (screenshot). **Drag from maximized (restore + follow pointer): FAIL**, two multi-point drags from the maximized title bar left the window maximized at 0,0 with no restore. **Edge-resize: not achieved**: the right-edge drag at (1235,500) hit the overlapping ferrite-run CTRL terminal of another session instead of the Ferrite border (no typing; Ferrite geometry unchanged). A quick two-point drag did not move the window, so the tool may need slow drags. The log has no winit geometry lines for these actions. | logs/ferrite-debug-20261009-045325-GIT-2.log (no geometry lines logged) | shots/TTL-6-a-dblclick-restore.png |
| EXTRA-YT-7 | FAIL (crash) | Doc `/tmp/batch12b/yt7.md` (copy of `test_md/test_video_embeds.md`, which has `{{video …}}` embeds #1 to #4 and bare links). Opened in Raw, then switched to **Rendered** (V button / Ctrl+E): Ferrite panicked at once with `GDK has not been initialized. Call gdk::init or gtk::init first.` at `gdk-0.18.2/src/auto/display.rs:301`, from `ferrite::markdown::video_render::create_child_webview` ← `ferrite::markdown::editor::render_video_embed` ← `render_node` ← `MarkdownEditor::show_rendered_editor`. Exit code 134 (SIGABRT). **Reproduced 2 times**: first in the GIT-2 instance (log logs/ferrite-debug-20261009-045325-GIT-2.log, panic at 05:01:39 CEST), then in a fresh instance (logs/ferrite-debug-20261009-050216-EXTRA-YT-7-fresh.log, panic at 05:02:39 CEST). The row stopped at the first step, so the video players, the Settings overlay, the reload dialog and the Search-in-Files panel were **not reached** and nothing is claimed for them. | logs/ferrite-debug-20261009-050216-EXTRA-YT-7-fresh.log (fail excerpt below) | shots/EXTRA-YT-7-fail.png (desktop after the crash; the Ferrite window is gone) |
### EXTRA-YT-7 (fresh instance, last 30 non-span lines; logs/ferrite-debug-20261009-050216-EXTRA-YT-7-fresh.log)
```
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 05:02:39 CEST ===
```

### Batch 12b continued: TTL-6b and MW-5

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| TTL-6b | PARTIAL | **ON step (via UI):** Settings opened from the custom title bar gear, then Appearance, then "Use system title bar" ticked. Help text: "Restart Ferrite for this change to take effect." The value was saved when that instance closed (03:03:53 UTC). After relaunch the custom title bar was gone and the xfwm4 native title bar showed (window title "test_video_embeds.md - Ferrite"). Double-click and the maximize button responded visually. Drag and edge-resize on the native decorations were **not separately recorded**. **OFF step (not done through the UI):** with the system title bar on, the gear is not drawn (it exists only in the custom title bar, `src/app/title_bar.rs`), and no other Settings entry point opened. Ctrl+, was received (log: "Keyboard shortcut: Ctrl+, (Open Settings)" at 03:04:52, 03:05:32 and 03:06:01 UTC), but no Settings tab appeared and Ctrl+Shift+P showed no palette. A status-bar icon opened About, and a toolbar icon opened the terminal panel (toggled back). Source note: `toggle_settings()` only flips `ui.show_settings`, and nothing reads that flag (`rg show_settings src`). **Workaround:** with Ferrite closed I set `"use_system_title_bar": false` in `~/.config/ferrite/config.json` line 15 (backup `/tmp/batch12b/config.json.ttl6b-on-backup`) and relaunched. **Confirmed:** the custom title bar came back (R/S/V pill and gear visible). Gear, then Settings, then Appearance showed "Use system title bar" unchecked. Closed with Alt+F4 (clean exit 0). | logs/ferrite-debug-20261009-050403-TTL-6b-on.log (ON); logs/ferrite-debug-20261009-050735-TTL-6b-off.log (OFF) | shots/TTL-6b-on-native-titlebar.png; shots/TTL-6b-pass.png |
| MW-5 | SKIPPED (no v1 session fixture) | Searched `test_md` and `fixtures` for v1/version files, then filtered for session: no matches. The only session files on the box are the live `~/.config/ferrite/session.json` and `session.recovery.json`. Both are `"version": 2`, and neither is a fixture. Nothing was hand-crafted. | none | none |

### TTL-6a (FAIL minor): last 30 non-span lines
Log: logs/ferrite-debug-20261009-045325-GIT-2.log. TTL-6a ran in this instance before the EXTRA-YT-7 crash, and the log has no geometry lines. The tail below is therefore dominated by the later crash.
```
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: gdk::auto::display::Display::default
   3: ferrite::markdown::video_render::create_child_webview
   4: ferrite::markdown::editor::render_video_embed
   5: ferrite::markdown::editor::render_node
   6: core::ops::function::FnOnce::call_once{{vtable.shim}}
   7: egui::ui::Ui::allocate_ui_with_layout_dyn
   8: core::ops::function::FnOnce::call_once{{vtable.shim}}
   9: egui::ui::Ui::scope_dyn
  10: core::ops::function::FnOnce::call_once{{vtable.shim}}
  11: egui::ui::Ui::scope_dyn
  12: core::ops::function::FnOnce::call_once{{vtable.shim}}
  13: ferrite::markdown::editor::MarkdownEditor::show_rendered_editor
  14: ferrite::markdown::editor::MarkdownEditor::show
  15: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel::{{closure}}
  16: egui::containers::panel::CentralPanel::show_inside_dyn
  17: ferrite::app::central_panel::<impl ferrite::app::FerriteApp>::render_central_panel
  18: ferrite::app::FerriteApp::render_ui
  19: <ferrite::app::FerriteApp as eframe::epi::App>::ui
  20: egui::context::Context::run_ui_dyn::{{closure}}
  21: egui::context::Context::run_dyn
  22: egui::context::Context::run_ui_dyn
  23: <eframe::native::glow_integration::GlowWinitApp as eframe::native::winit_integration::WinitApp>::run_ui_and_paint
  24: <eframe::native::run::WinitAppWrapper<T> as winit::application::ApplicationHandler<eframe::native::winit_integration::UserEvent>>::window_event
  25: core::ops::function::impls::<impl core::ops::function::FnMut<A> for &mut F>::call_mut
  26: eframe::native::run::run_glow
  27: ferrite::main
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
=== Ferrite exited with code 134 SIGABRT - panic (release panic=abort) => FAIL at 05:01:39 CEST ===
```

### Batch 12b final state
- Use system title bar: OFF (config.json line 15 is `false`; confirmed in Settings).
- code_execution_consent_acknowledged: `true`, which matches the original (/tmp/batch12b/config.json.orig).
- Ferrite quit with Alt+F4 (clean exit 0 at 05:08:21 CEST). `pgrep -a -x ferrite`:
```
empty
```
`git -C /workspace/ferrite-qa/ferrite status --short` output:
```
<empty>
```

## Batch 12c (Settings persistence remainder)

Tester: Grok Bot (QA only, no code changes, no commits/PRs/issues). Rows: EXTRA-SET-5, -6, -7, -13, -14 (gui-test-plan.md Batch 3 EXTRA-SET rows). Launcher: `/workspace/ferrite-qa/run-ferrite.sh --tag <ROW> /workspace/ferrite-qa/ferrite/README.md`. Config backup before the batch: `logs/config-before-12c.json` (restored at the end with Ferrite closed). Started: Fri Oct 9 2026 ~05:10 CEST.

| ROW-ID | RESULT | observation | log | screenshot |
|---|---|---|---|---|
| EXTRA-SET-5 | PARTIAL | Changed all 10 Settings > Terminal entries in one instance (run tag EXTRA-SET-5, 05:09): font size 14→32px (slider max); font Default→Custom System Font "Abel"; scrollback 10000→50000; Copy Selection ON; theme Ferrite Dark→Dracula; Background Opacity 100%→10% (slider min); breathing colour blue→yellow (picker); Auto-load Layout ON→OFF; Enable Sound on Prompt OFF→ON (custom file left empty); Focus Terminal on Prompt OFF→ON. Alt+F4 = clean exit 0 at 05:12:35. `python3 -m json.tool ~/.config/ferrite/config.json` rc=0 and the file holds all 10 new values (terminal_font_size 32.0, terminal_font_family Abel, terminal_scrollback_lines 50000, terminal_copy_on_select true, terminal_theme_name Dracula, terminal_opacity 0.1, terminal_breathing_color [237,223,100,255], terminal_auto_load_layout false, terminal_sound_enabled true, terminal_focus_on_detect true). Relaunch (tag EXTRA-SET-5-relaunch, 05:12:46, no Session Recovery dialog): Settings shows all values (Recently Changed + Terminal filter; OK). Terminal panel opened from ribbon: visible, typing `echo hello12c` + Enter echoed and printed output; at 10% opacity text is very faint but still readable/usable. **Persistence: all 10 entries persisted. DEFECT: "Auto-load Layout" OFF did not stop the workspace layout from loading.** On the first panel open the log shows `Loaded workspace layout from .../.ferrite/terminal-layout.json` then `Terminal panel initialized from workspace layout` (03:13:18Z). Cause: `TerminalPanelState::toggle()` calls `initialize()` (src/ui/terminal_panel.rs ~L152), which calls `try_load_workspace_layout()` (~L344) with no check of `terminal_auto_load_layout`. The only gate is in src/app/navigation.rs:431. Side note: the Recently Changed list keeps only 8 entries (most recent first); Auto-load and Sound fell off it. | logs/ferrite-debug-20261009-050949-EXTRA-SET-5.log (change+quit); logs/ferrite-debug-20261009-051246-EXTRA-SET-5-relaunch.log (relaunch, panel open) | shots/EXTRA-SET-5-fail.png (panel open at 10% opacity, Dracula, Abel 32px) |
| EXTRA-SET-6 | PARTIAL | Rebind + restart + triggers: Toggle Outline→F7, Toggle Word Wrap→F8, Toggle Terminal→Ctrl+Shift+J (run 05:15). After Alt+F4 + relaunch (05:20:55) the three bindings were still shown in Settings > Keyboard and each key toggled its command (F7 Outline 03:21:08Z, F8 Word Wrap 03:21:15Z, Ctrl+Shift+J Terminal 03:21:44Z; Ctrl+Shift+J needed 3 tries to register). Esc during capture cancelled (03:15:45Z), modifier-only Shift press did not bind. **Ctrl+Alt+Shift combo: BLOCKED** after 4 attempts on Toggle Outline / Toggle Productivity Hub: Ctrl+Alt+Shift+O was captured as "0"; Ctrl+Alt+Shift+K was captured as bare "K" (Alt dropped) and bound Toggle Outline to plain K (log 03:17:23Z: `K (Toggle Outline)` fired, and `Ctrl+Shift+K (Image)` fired in the same press, so Alt never reached Ferrite); Ctrl+Shift+Alt+K left the capture at "Waiting for key..."; 4th try (05:23) hit a list-scroll mismatch and changed nothing. Likely xdotool / xfwm4 dropping Alt, not verifiable from Ferrite side. **Reset All Shortcuts** (Settings > Keyboard button) restored defaults in the UI (Word Wrap Alt+Z, Outline Ctrl+Shift+O, Terminal Ctrl+`, Productivity Hub Ctrl+Shift+H, no override markers); config.json `keyboard_shortcuts` became `{"bindings": {}}` after Alt+F4 (clean exit 05:23:38); after relaunch (05:23:55) defaults still shown. Overall: persistence and triggers OK; Ctrl+Alt+Shift path blocked by the Alt-modifier loss above. | logs/ferrite-debug-20261009-051502-EXTRA-SET-6.log (rebind/capture, 03:17:23Z Alt-loss line); logs/ferrite-debug-20261009-052055-EXTRA-SET-6-relaunch.log (restart + triggers); logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log (after reset, defaults) | shots/EXTRA-SET-6-defaults.png (Keyboard list after reset + restart) |
| EXTRA-SET-7 | PARTIAL | Language dropdown (Settings > Appearance > Language, top of list) offers only **5** languages: English, Chinese (Simplified), German, Spanish, Japanese (`Language::all()` in src/config/settings.rs:1134). **cs, et, nb_NO, pt, ru are not offered** even though `locales/` has yaml files for them and the row expected them; SKIPPED for those 5 (not selectable). Switched through the 5 offered ones, each applied immediately: zh-Hans UI fully translated, CJK glyphs render, no raw keys in Appearance or Keyboard lists, no tofu, no clipping seen. German and Spanish fully translated, no clipping (Alles zurücksetzen / Restablecer todo fit). Japanese: Appearance and most Keyboard labels translated; several Keyboard labels fall back to English (Toggle Word Wrap, Lock editing, Toggle Terminal, Toggle Productivity Hub, Zoom In, Reset Zoom). In all non-English UIs "Use system title bar" and "Restart Ferrite for this change to take effect." stay English. Log WARN `Failed to find replacement characters '◻' or '?'` at Japanese font load (03:26:07Z), no visible glyph loss in screenshot. Restart test: language Japanese persisted (config `language: ja`, Recently Changed shows Language=Japanese after Alt+F4 + relaunch 05:26:38); switched back to English in the same session (English shown after switch). Russian not offered, so no ru screenshot. | logs/ferrite-debug-20261009-052355-EXTRA-SET-6-reset.log (switches, Japanese session, quit 05:26:29); logs/ferrite-debug-20261009-052638-EXTRA-SET-7-ja-restart.log (restart in Japanese, switch back to English) | shots/EXTRA-SET-7-zh_Hans.png, shots/EXTRA-SET-7-ja.png (no ru: not offered) |
| EXTRA-SET-13 | PASS | Ctrl+Shift+N opened a second window (window 2, "Untitled - Ferrite") but only on the 5th keypress: presses 1-4 (Settings tab focused, then after clicking the Settings panel, then once more) left no `Keyboard shortcut: Ctrl+Shift+N` line in the log; the 5th press, with the README.md tab clicked first, opened it (log `create_window` 03:28:40Z). In window 2 (Settings via its own gear): Theme Light→Dark (03:29:07Z `Theme changed from Light to Dark`) and Font Size 14px→Large (18px). **Window 1 reflected both live, no restart:** dark background (sampled 30,30,30) and larger editor text (I did not inspect window 1's own Settings tab). Closed window 2 with its title-bar X (app kept running in window 1). Full restart (Alt+F4 clean exit 05:30:04, relaunch 05:30:16): dark theme and 18px font present, config `theme: dark`, `font_size: 18.0`; recently_changed list shows appearance.theme and appearance.font_size. Side effect: I accidentally toggled the title-bar focus/zen icon (x=1135) once in window 1 and reverted it. | logs/ferrite-debug-20261009-052638-EXTRA-SET-7-ja-restart.log (window 2 create, theme change at 03:29:07Z, close); logs/ferrite-debug-20261009-053015-EXTRA-SET-13-restart.log (full restart, dark 18px) | shots/EXTRA-SET-13-restart-check.png (after full restart: dark, 18px) |
| EXTRA-SET-14 | PARTIAL | Settings > "Reset All" (top right, not the keyboard-only one) was applied on a single click: **no confirmation dialog appeared**; the status bar showed "Settings reset to defaults" and every value went to default immediately (Theme Light, Font 14px, Language English, Terminal Font default, Scrollback 10000, Terminal Font Size 14, Terminal Theme Ferrite Dark, Opacity 100%). Code agrees: the button sets `reset_requested` with no confirm step (src/ui/settings/mod.rs ~L225). **Recently Changed:** the list was NOT cleared in-session. The 8 entries stayed visible after Reset All, now showing default values. It cleared on quit: config `recently_changed_settings: []`, and after relaunch the Settings page starts at "Essentials" with no Recently Changed section. Quit (Alt+F4) clean exit 05:31:46; config after exit: theme light, font_size 14, terminal_theme_name Ferrite Dark, terminal_font_size 14, scrollback 10000, opacity 1.0, font family None, copy_on_select false, auto_load_layout true, sound false, focus_on_detect false, breathing [100,149,237,255]; keyboard_shortcuts `{"bindings": {}}`. Relaunch (05:31:58) showed defaults in effect (Light, 14px, English). | logs/ferrite-debug-20261009-053015-EXTRA-SET-13-restart.log (Reset All and quit 05:31:46); logs/ferrite-debug-20261009-053158-EXTRA-SET-14-relaunch.log (defaults after restart) | shots/EXTRA-SET-14-defaults.png (defaults after relaunch, Settings top) |

**Batch 12c teardown:** Ferrite quit (Alt+F4, clean exit 05:32:47 CEST). Config restored: `cp logs/config-before-12c.json ~/.config/ferrite/config.json` (cmp identical, python -m json.tool OK). `pgrep -a -x ferrite` empty. `git -C /workspace/ferrite-qa/ferrite status --short` output: (empty). Rows reached: EXTRA-SET-5, -6, -7, -13, -14 (all five). Not reached: none. Results: 5 = PARTIAL (auto-load layout defect), 6 = PARTIAL (Ctrl+Alt+Shift BLOCKED), 7 = PARTIAL (only 5 of 10 languages offered), 13 = PASS, 14 = PARTIAL (no confirm step, in-session Recently Changed not cleared). No FAIL / no hang / no panic (exit 134 never seen; grep "panicked" only matched the GL "NO_ERROR" string).
