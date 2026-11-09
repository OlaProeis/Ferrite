# Ferrite v0.3.1 — GUI test plan for the desktop operator (Linux box)

Source of truth: `docs/technical/platform/v0.3.1-grok-bot-qa-brief.md` (the "brief"),
`v0.3.1-minimal-release-test.md` (the "minimal test"), `v0.3.1-total-test.md`
(the "total test"). This plan only re-orders and translates those documents to
Linux; where it adds rows they are marked **EXTRA** (Ola's priority request:
YouTube embeds and the new Settings panel). Expected results come from the
checklists; for EXTRA rows they come from the code where stated, otherwise
"record behaviour" (no crash is the hard requirement).

- Repo clone: `/workspace/ferrite-qa/ferrite` — branch `0.3.1-experimental`,
  commit `8e6ef508aef141b243b961e278861d76f8bc0d48`
- Release binary: `/workspace/ferrite-qa/ferrite/target/release/ferrite`
  (abbreviated `$FER` below: `FER=/workspace/ferrite-qa/ferrite/target/release/ferrite`)
- Launcher: `/workspace/ferrite-qa/run-ferrite.sh` (always use it, so the debug
  log + exit code are captured in `/workspace/ferrite-qa/logs/ferrite-debug-<ts>[-tag].log`)
- Fixture helper: `/workspace/ferrite-qa/prep-fixtures.sh` (re-run = reset fixtures)
- EOL/encoding checker: `/workspace/ferrite-qa/eol-check.sh FILE crlf|lf|utf16|cp1252`
- Screenshots: `/workspace/ferrite-qa/shots/<ROW-ID>-<pass|fail>.png`
- Report: `/workspace/ferrite-qa/qa-report-v0.3.1-linux.md`, format in
  `/workspace/ferrite-qa/report-format.md` (brief §7, verbatim)

---

## 0. Ground rules (from the brief — non-negotiable)

1. **Tester, not fixer.** No code changes, no commits/pushes/PRs/issues. Do not
   work around a crash to make a row pass.
2. **A crash is always FAIL.** Release builds use `panic = "abort"`: the window
   just disappears. The launcher prints `exited with code 134 … SIGABRT - panic`.
   For every FAIL keep the log file and copy the last ~30 log lines into the report.
3. Can't do a row (platform / network / hardware) → `SKIPPED (<reason>)`. Stuck
   after ~4 attempts → `BLOCKED` with what you saw. **Never invent a result.**
4. **Stale notes are obsolete.** Any checklist note "KNOWN CRASH / KNOWN BUG /
   KNOWN GAP until PRD … lands" no longer applies — those PRD tasks landed.
   Rows **4b, 15, 16, 21, 25b, 38** must now **PASS**; a crash or wrong result
   there is a real FAIL (severity `blocker` in Tier 1).
5. Screenshots: **every FAIL** + the Tier 1 PASS rows marked "Shot: YES" below.
   Name `<ROW-ID>-<pass|fail>.png` (IDs: `T1-<n>` / `T2-<n>` / `T3-<n>` for the
   minimal test, checklist IDs like `EOL-7`, §6 IDs like `SIF-1`, `EXTRA-YT-3`).
   For terminal-verified rows (EOL/hex checks) also save the checker output:
   `eol-check.sh … | tee /workspace/ferrite-qa/logs/<ROW-ID>-check.txt`.
6. Tier 1 failure → keep going, mark it `blocker`.
7. If you run low on budget: finish through the §6 extra rows (Batch 8) and write
   the report, listing the total-test sections not reached (brief §8).

## 1. Environment facts the operator must know

- **Display.** The box runs one Xvfb per agent desktop (seen: `:1` and `:2`,
  1920×1200×24, GLX via Mesa software rendering, xfwm4 WM, compositor off, no
  Wayland). The launcher uses **your** `$DISPLAY` — run it from a terminal on
  your own desktop. Software GL is already the reality; if the window does not
  appear use `run-ferrite.sh --soft-gl --x11 …` (brief §1).
- **One Ferrite "primary" per box user.** Single-instance state lives in
  `~/.config/ferrite/` (`instance.lock/port/pid`), which is shared by every agent
  on this box. Before starting, make sure no other Ferrite is running:
  `pgrep -a ferrite` (should print nothing). Otherwise your launch is forwarded
  to someone else's window and exits.
- **RUST_LOG is ignored** by Ferrite (logger uses an explicit filter); verbosity
  comes from `--log-level debug`, which the launcher passes. A forwarded second
  instance exits before logging starts → its log is (nearly) empty; that's normal.
- **Debug logs are noisy and big** (~1.6 MB per 20 s idle, mostly winit
  `tracing::span` lines). For the report's "last ~30 log lines" use
  `grep -v 'tracing::span' LOG | tail -n 30`. Exit codes printed by the launcher:
  0 clean · 134 SIGABRT = panic (FAIL) · 137 kill -9 · 139 segfault (FAIL) · 143 SIGTERM.
  Prep sanity launch: window "test.md - Ferrite" 1200×800 appeared, renderer
  glow / OpenGL 4.5 Mesa llvmpipe; benign warnings seen at startup: XSETTINGS,
  XRandR "0mm" display size, epaint "Failed to find replacement characters '◻' or '?'".
- **Clipboard (X11):** image → `xclip -selection clipboard -t image/png -i /workspace/ferrite-qa/ferrite/test_md/assets/sample.png`;
  text → `printf 'https://example.com' | xclip -selection clipboard`;
  read back → `xclip -selection clipboard -o`. Primary selection (middle-click):
  `xclip -selection primary -o` / `printf 'abc' | xclip -selection primary`.
- **External edits** (checklist "Notepad"): from a second terminal,
  `printf 'external edit\n' >> FILE` or `sed -i 's/First/FIRST/' FILE`.
- **Kill (Task Manager):** `kill -9 $(pgrep -x ferrite)`.
- **Alt+F4** works under xfwm4; otherwise the `×` at top-right of Ferrite's own title bar.
- **File manager** for "Reveal"/UX-9: `xdg-open` + `thunar` are installed. If
  nothing opens → `SKIPPED (no file manager)`, not FAIL.
- **Network:** at prep time the box reached `img.youtube.com`,
  `www.youtube-nocookie.com` and `www.youtube.com` (HTTP 200), so video rows
  are expected to be testable. WebView = WebKitGTK 4.1 (2.54) via wry.
- **Config dir:** `~/.config/ferrite/` (settings file `config.json`, backup
  `config.json.bak`, spellcheck user words `spellcheck/user-words.txt`). It did
  not exist before QA. The prep sanity launch used a throw-away
  `XDG_CONFIG_HOME`, but **`cargo test` itself created `~/.config/ferrite/config.json`
  + `recovery/` at 20:06 (the unit tests write to the real config dir)** — so do
  Batch 0 step 2 (move it aside) before the first GUI launch.
- **Do not leave edits in the repo fixtures between rows that need a pristine
  file.** Restore one fixture with
  `git -C /workspace/ferrite-qa/ferrite checkout -- test_md/<file>`; check with
  `git -C /workspace/ferrite-qa/ferrite status --short`. (Restoring fixtures is
  not a code change; never commit.) Record `git status --short` at the end of
  the session in the report's notes.
- **Default shortcuts (from code, `src/config/settings.rs`):** Settings **Ctrl+,** ·
  Command palette **Alt+Space** · Quick open **Ctrl+P** · New doc **Ctrl+N** · New tab **Ctrl+T** ·
  New window **Ctrl+Shift+N** · Reload **Ctrl+Shift+R** · Delete line **Ctrl+D** ·
  Duplicate **Ctrl+Shift+D** · Move line **Alt+↑/↓** · TOC **Ctrl+Shift+U** · Redo **Ctrl+Y** ·
  Find **Ctrl+F** / Replace **Ctrl+H** · Search in files **Ctrl+Shift+F** · Word wrap **Alt+Z** ·
  Preview lock **Insert** · Toggle view mode **Ctrl+E** · Terminal **Ctrl+`** · About **F1** ·
  Outline **Ctrl+Shift+O** · File tree **Ctrl+Shift+E**. (Ctrl+Shift+P is Export PDF, not the palette.)
- **Code facts that may differ from checklist text (record what you SEE, do not
  "correct" it):** (a) Settings "Recently changed" cap is **8** in code
  (`MAX_RECENTLY_CHANGED = 8`), the minimal test row 25 says "max 6";
  (b) keyboard shortcut rebinds are deliberately **not** added to "Recently
  changed" (code comment), while row 25 lists "one shortcut". Report the
  observed behaviour and flag the mismatch in Notes.

---

## Batch 0 — Session setup (no result rows)

1. `pgrep -a -x ferrite` → must be empty (kill stray QA instances you own if not).
2. Clean first launch (brief §1): `[ -d ~/.config/ferrite ] && mv ~/.config/ferrite ~/.config/ferrite.bak-$(date +%s)`.
3. `/workspace/ferrite-qa/prep-fixtures.sh` (creates `/tmp/crlf-test.md`,
   `/tmp/lf-test.md`, `/tmp/crlf-utf16.md`, `/tmp/cp1252.md`, `/tmp/mixed-eol.md`,
   `/tmp/crlf-video.md` and `/workspace/ferrite-qa/fixtures/*`).
4. `export FER=/workspace/ferrite-qa/ferrite/target/release/ferrite`
5. Launch: `/workspace/ferrite-qa/run-ferrite.sh --tag session1 /workspace/ferrite-qa/ferrite`
   (repo as workspace). Pre-flight (total test): app reaches Welcome / main
   window within ~3 s; Help/About shows `0.3.1`. Note anything odd.
6. Keep a second terminal open for external edits, clipboard and hex checks.

---

## Batch 1 — Tier 1 part 1 (rows 1–11: line endings, reload, exit safety) — 12 rows

### T1-1 (minimal row 1) — CRLF: click+Escape in Rendered does not dirty
- Doc: `/tmp/crlf-test.md` (fresh: `prep-fixtures.sh eol`)
- Steps: open it (`run-ferrite.sh /tmp/crlf-test.md` or, if Ferrite is already running, forward it from a 2nd terminal: `$FER /tmp/crlf-test.md`) → switch to **Rendered** → click "First paragraph." → press **Escape** without typing.
- Expected: tab title has **no `*`** (not dirty).
- Shot: **YES** (`T1-1-pass.png` showing the clean tab title).

### T1-2 (row 2) — CRLF kept after Rendered edit
- Doc: `/tmp/crlf-test.md` (continue from T1-1)
- Steps: click the paragraph again, type ` edited`, click elsewhere → **Ctrl+S** → terminal: `eol-check.sh /tmp/crlf-test.md crlf | tee logs/T1-2-check.txt`.
- Expected: `RESULT: CRLF preserved (True)` (every line ends in CR; `file` says "with CRLF line terminators").
- Shot: FAIL only (+ check txt always).

### T1-3 (row 3) — CRLF kept after Raw Enter/Backspace join
- Doc: `/tmp/crlf-test.md`
- Steps: **Raw** → caret at end of "Second paragraph with more words." → **Enter** → type `new line` → caret to start of that new line → **Backspace** (joins lines) → **Ctrl+S** → `eol-check.sh /tmp/crlf-test.md crlf`; also `cat -A /tmp/crlf-test.md` (Linux stand-in for "open in Notepad").
- Expected: CRLF True; no stray `^M` mid-line, no doubled blank lines.
- Shot: FAIL only.

### T1-4 (row 4) — CRLF kept after line ops + TOC
- Doc: `/tmp/crlf-test.md`
- Steps: Raw: **Alt+↑** / **Alt+↓** move a line, **Ctrl+Shift+D** duplicate it, **Ctrl+Shift+U** insert TOC → **Ctrl+S** → `eol-check.sh /tmp/crlf-test.md crlf`.
- Expected: CRLF True (also covers total-test EOL-6 and EOL-8 — check the TOC lines end in `0d0a` in the xxd output).
- Shot: FAIL only.

### T1-4b (row 4b) — UTF-16 LE CRLF round trip  **(stale KNOWN BUG note → must PASS)**
- Doc: `/tmp/crlf-utf16.md` (made by prep script per brief §4: BOM `fffe` + `iconv … UTF-16LE`)
- Steps: open in Ferrite → **Raw** → press **Enter** once (anywhere) → **Ctrl+S** → `eol-check.sh /tmp/crlf-utf16.md utf16 | tee logs/T1-4b-check.txt`.
- Expected: only `0d00 0a00` pairs, **no lone `0a00`**; text still readable in Ferrite (no mojibake).
- Shot: **YES** (pass: Ferrite + terminal check output; fail: same).

### T1-5 (row 5) — Reload from Disk, clean tab
- Doc: `/workspace/ferrite-qa/fixtures/reload-test.md`
- Steps: open it, don't edit → 2nd terminal: `sed -i 's/First/FIRST/' /workspace/ferrite-qa/fixtures/reload-test.md` → back in Ferrite **Ctrl+Shift+R**.
- Expected: content updates to "FIRST line…", **no dialog**.
- Shot: FAIL only.

### T1-6 (row 6) — Reload, dirty tab: Cancel and Don't Save
- Doc: same file
- Steps: type something in Ferrite (tab dirty) → terminal: `printf 'external edit\n' >> /workspace/ferrite-qa/fixtures/reload-test.md` → **Ctrl+Shift+R** → dialog → **Cancel** (your edit stays) → **Ctrl+Shift+R** again → **Don't Save**.
- Expected: dialog appears; Cancel keeps your edit; Don't Save shows disk content (incl. "external edit").
- Shot: **YES** (the confirm dialog).

### T1-7 (row 7) — Reload in Rendered with uncommitted edit
- Doc: same file
- Steps: **Rendered** → click into a paragraph → type 2 chars → **do not** click away → **Ctrl+Shift+R**.
- Expected: confirm dialog appears (edits not silently dropped). (= total RLD-8)
- Shot: **YES**.

### T1-8 (row 8) — Reload dialog targets the original tab
- Docs: `/workspace/ferrite-qa/fixtures/reload-test.md` + `/workspace/ferrite-qa/fixtures/other-tab.md`
- Steps: both open; make reload-test dirty + external edit (`printf 'ext2\n' >> …/reload-test.md`) → **Ctrl+Shift+R** on reload-test (dialog open) → with the mouse **close the other-tab tab** → confirm the reload (Don't Save).
- Expected: reload-test reloads (shows `ext2`); other-tab untouched on disk (`cat …/other-tab.md`) and its tab simply closed.
- Shot: FAIL only.

### T1-9 (row 9) — Exit with Save saves all tabs
- Docs: `/workspace/ferrite-qa/fixtures/save-a.md`, `save-b.md`
- Steps: open both, edit both → **Alt+F4** (or title-bar `×`) → **Save**.
- Expected: both files updated on disk (`cat` both), app closes, dialog does not loop. Launcher prints exit code 0.
- Shot: FAIL only.

### T1-10 (row 10) — Exit with Don't Save leaves nothing behind
- Doc: `/workspace/ferrite-qa/fixtures/discard-test.md`
- Steps: relaunch with the file, edit → **Alt+F4** → **Don't Save** → relaunch `run-ferrite.sh`.
- Expected: discarded edit does **not** come back; no "Recovered content differs" banner, no autosave dialog; disk still `original content`.
- Shot: FAIL only.

### T1-11 (row 11) — Kill → recovery → Restore → one Ctrl+Z
- Doc: `/workspace/ferrite-qa/fixtures/kill-test.md`
- Steps: open, type an edit, wait ~5 s → terminal `kill -9 $(pgrep -x ferrite)` → relaunch → recovery dialog → **Restore** → **Ctrl+Z** once.
- Expected: recovery dialog offers Restore; restoring shows your edit; one Ctrl+Z returns to disk content. (= total SES-7)
- Shot: **YES** (recovery dialog).

---

## Batch 2 — Tier 1 part 2 (rows 12–22: multibyte, rendered edit, mermaid) — 13 rows

### T1-12 (row 12) — Multibyte at line start (#171)
- Doc: new tab (**Ctrl+N**)
- Steps: type `§` as the first char of a line, Enter, `é`, Enter, `中`.
- Expected: no crash. (covers STAT-1/2)
- Shot: FAIL only.

### T1-13 (row 13) — Line ops on CJK/accents
- Doc: `/workspace/ferrite-qa/fixtures/t1-multibyte.md` (line `日本語テキスト ééé`)
- Steps: Raw, caret at end of that line → **Ctrl+Shift+D**, **Alt+↑**, **Alt+↓**, then delete line (**Ctrl+D** = default Delete Line binding).
- Expected: no crash, caret stays sane. (covers LOPS-1..3)
- Shot: FAIL only.

### T1-14 (row 14) — Clipboard image paste after CJK (#164)
- Doc: same file (saved, so `assets/` goes next to it)
- Steps: caret at end of `日本語テキスト ééé` → terminal: `xclip -selection clipboard -t image/png -i /workspace/ferrite-qa/ferrite/test_md/assets/sample.png` → **Ctrl+V** → switch to Rendered.
- Expected: `![](assets/…png)` inserted at the caret; PNG exists in `/workspace/ferrite-qa/fixtures/assets/` (`ls`); image renders in Rendered. (covers IMG-3, IMG-9)
- Shot: **YES**.

### T1-15 (row 15) — Search in Files `中文`  **(stale KNOWN CRASH note → must PASS)**
- Doc: workspace `/workspace/ferrite-qa/ferrite` (launch with that path). CJK file: create `/workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md` containing `搜索测试 中文 search` (also used by SIF rows; delete it after SIF-4).
- Steps: **Ctrl+Shift+F** → query `中文` → click the result for `zz_cjk_search.md`.
- Expected: no crash; result listed; caret lands **on** `中文`, not after it.
- Shot: **YES**.

### T1-16 (row 16) — Find whole-word with CJK / é  **(stale KNOWN CRASH note → must PASS)**
- Doc: `/workspace/ferrite-qa/fixtures/t1-multibyte.md` (has `搜索测试 中文 search` and `éa é`)
- Steps: Raw → **Ctrl+F** → enable **Whole word** → query `中文` → then enable **Whole word + Case sensitive** → query `é`.
- Expected: `中文` found, no crash; `é` query matches only the standalone `é` in `éa é`, no crash.
- Shot: **YES**.

### T1-16b (row 16b) — Smart URL paste over selection after CJK
- Doc: same file (line `日本語日本語 hello world`)
- Steps: select `hello` → terminal `printf 'https://example.com' | xclip -selection clipboard` → **Ctrl+V**.
- Expected: `[hello](https://example.com)`, no crash.
- Shot: FAIL only.

### T1-17 (row 17) — Arrow-nav to off-screen paragraph keeps it whole
- Doc: `/workspace/ferrite-qa/ferrite/test_md/test_find_arrow_nav.md`
- Steps: Rendered → click the first heading → **ArrowDown** ~15× until a multi-line paragraph that started off-screen → type one char → **Escape** → **Raw**.
- Expected: that paragraph complete (not truncated to its first line); only one char added. (= ARR-6 first half)
- Shot: **YES** (Raw view of the paragraph).

### T1-18 (row 18) — Edit inside blockquote doesn't merge the quote
- Doc: same file (blockquote around line 32, Section 3)
- Steps: arrow-nav onto a block inside the blockquote → type a char → Escape → Raw.
- Expected: only that block changed; quote lines not merged into one line.
- Shot: FAIL only.

### T1-18b (row 18b) — Container prefixes survive rendered edits
- Doc: new tab, type exactly (blank lines between):
  `> ## Title` / `` / `> quoted one` / `` / `> [!NOTE]` + `> callout body` → save as `/workspace/ferrite-qa/fixtures/t1-18b.md`
- Steps: Rendered → click each block, type `x`, click elsewhere → Raw.
- Expected: `> ## Titlex`, `> quoted onex`, `> callout bodyx`. `# Titlex` or `> > quoted onex` = regression (FAIL).
- Shot: **YES** (Raw result).

### T1-19 (row 19) — Split undo on code block (#167)
- Doc: `/workspace/ferrite-qa/ferrite/test_md/test_copy_and_split_undo.md` Part B (line ~27)
- Steps: **Split** → edit the code block in the preview → click away → **Ctrl+Z** → then 3× (Ctrl+Y, Ctrl+Z) cycles.
- Expected: only that edit reverts; `SENTINEL` lines intact after 3 undo/redo cycles. (covers UND-1/2)
- Shot: **YES**. Afterwards: `git -C /workspace/ferrite-qa/ferrite checkout -- test_md/test_copy_and_split_undo.md` if saved.

### T1-20 (row 20) — Selection + Alt+click caret → Tab
- Doc: `/workspace/ferrite-qa/ferrite/test_md/test_indent_selection.md`
- Steps: Raw → select 3 lines (Block 1) → **Alt+click** a caret on a 5th line → **Tab** → then **Ctrl+Z** once.
- Expected: every target indented, no text moved into the wrong line; one Ctrl+Z reverts the whole indent. (covers IND-6, IND-10)
- Shot: **YES**.

### T1-21 (row 21) — Mermaid typed char-by-char never kills the app  **(stale KNOWN CRASH note → must PASS)**
- Doc: new tab, **Split**
- Steps: type character by character:
  ```` ```mermaid ```` / `flowchart TD` / `A[Start] --> B{Decide}` / `B -->|yes| C` / `subgraph 业务 PEP` / `C --> D` / ```` ``` ````
  then, inside a mermaid fence, also type the exact lines `A}x{ --> B` and `subgraph a] [b`.
- Expected: validation banner may flicker, app **never** closes; finished fence renders a diagram; the two malformed lines produce at most an error banner, **no crash** (launcher exit code must not be 134). (covers MMD-12)
- Shot: **YES** (rendered diagram; plus the malformed-input state).

### T1-22 (row 22) — Mermaid #165 fit, popup, layout
- Doc: `/workspace/ferrite-qa/ferrite/test_md/test_mermaid_issue_165.md`
- Steps: Rendered → widest flowchart fits pane (no horizontal scrollbar) → hover → **Fit / Native** toggle → click diagram → popup: zoom in/out/reset, close → check content right below the diagram (no huge gap) → scroll so a fit-mode diagram is half out of view → wheel-scroll with pointer over the diagram.
- Expected: as listed; nothing paints over the status bar; no jitter. Jitter/overpaint alone = cosmetic (PRD §6.6), log `minor`, not blocker. (covers MMD-8)
- Shot: **YES**.

---

## Batch 3 — FOCUS: Settings panel (Tier 2 rows 23–29 + related) + EXTRA-SET rows

Tag gate item 2: rows 23–29 must pass (no automated UI coverage). Settings
registry entries (from `src/ui/settings/registry.rs`) — use this list for the
EXTRA persistence sweep:

| Section | Entries (UI label) |
|---|---|
| Appearance | Theme · Ferrite accent · Language · Default View Mode · Font · Rendered view font · Font Size · Syntax Theme · Use system title bar · Additional Scripts · CJK Regional Preference |
| Editor | Word Wrap · Show Line Numbers · Show Minimap · Highlight Matching Brackets · Auto-close Brackets & Quotes · Syntax Highlighting · Use Spaces for Indentation · Tab Size · Vim Mode · Spellcheck · Middle-click paste · Strict Line Breaks · Maximum Line Width · LSP (Language Servers) · Code execution · Code Folding · Snippets · Paragraph Indentation · Header Spacing |
| Files | Restore previous session on startup · Show welcome on empty launch · Allow multiple Ferrite windows/instances · Quick note workflow · Enable Auto-Save by Default · Recent Files |
| Keyboard | Keyboard (all shortcut rows) |
| Terminal | Enable Integrated Terminal · Font Size · Terminal Font · Scrollback Lines · Copy Selection Automatically · Terminal Theme · Background Opacity · Startup Command · Detected Monitors · Breathing Indicator Color · Custom Prompt Patterns · Auto-load Layout · Sound Notification · Auto-Focus on Input |
| (search-only) | About |

### T2-23 (row 23) — Settings search
- Steps: **Ctrl+,** → type `zoom` → then `bold` → **Esc**.
- Expected: `zoom` shows the Font Size row; `bold` shows the Bold shortcut row; Esc clears the query.
- Shot: FAIL only (screenshot recommended for focus area).

### T2-24 (row 24) — Section chips + live counts
- Steps: empty query → click chips **Appearance / Editor / Files / Keyboard / Terminal** → then query `tab`.
- Expected: each chip shows its section; with `tab` the chips show live counts.
- Shot: FAIL only.

### T2-25 (row 25) — Recently changed persists
- Steps: change theme + font size + one shortcut → close and reopen Settings → check **Recently changed** → quit (Alt+F4) → relaunch → reopen Settings.
- Expected (checklist): listed (max 6), still listed after restart. **Note**: code caps at 8 and excludes shortcut rebinds — record exactly what is listed and the max you can get it to show.
- Shot: FAIL only.

### T2-25b (row 25b) — Widget identity in overview  **(stale KNOWN BUG note → must PASS)**
- Steps: overview visible (empty search): **drag** Font Size slider 14→20 in one motion → open the **accent colour** picker, pick 3 colours → type 5 characters into **Terminal startup command**.
- Expected: slider follows the pointer the whole way; picker popup stays open across 3 picks; all 5 chars land.
- Shot: FAIL only (screenshot recommended).

### T2-26 (row 26) — About
- Steps: About button (corner) → read version → **Check for updates** → click links.
- Expected: version `0.3.1`; update check works or shows an error toast (offline); links open browser (if no browser opens → note, `SKIPPED (no browser)` for that sub-step only).
- Shot: FAIL only.

### T2-27 (row 27) — Keyboard rebinding + cheat-sheet deep-link
- Steps: Keyboard section: rebind **Reload from Disk** → try an already-used chord (e.g. Ctrl+S) → **Reset** → then open the markdown **MD** cheat-sheet overlay (ribbon) → click a shortcut.
- Expected: conflict warning on used chord; Reset restores default (Ctrl+Shift+R); cheat-sheet click opens Settings with search prefilled on that row. (covers RLD-6, UX-6)
- Shot: FAIL only.

### T2-28 (row 28) — Ctrl+F scope + Reset All
- Steps: with a document tab active → **Ctrl+F** → document Find opens (not settings search) → Settings → **Reset All** → confirm.
- Expected: document find opens; defaults restored; no crash.
- Shot: FAIL only.

### T2-29 (row 29) — Skim all sections
- Steps: open every section once (all chips), scroll through.
- Expected: no raw locale keys (e.g. `settings.search_hint`); 0.3.0 controls all present: theme, accent, language, view mode, font picker, word wrap, line numbers, minimap, brackets, auto-save, quick note, code execution, terminal shell, native title bar.
- Shot: FAIL only.

### TTL-6 (total §11, Linux) — System title bar setting + custom title bar sanity
- Steps: (a) Default custom title bar: double-click → maximize; double-click → restore; drag; drag from maximized → restores and follows; edge-resize (this is also where minimal row 40 is reported on Linux). (b) Settings → Appearance → **Use system title bar** ON → restart if prompted → xfwm4 decorations appear and work → OFF again.
- Expected: setting works on Linux. Failures here are `minor` per brief §5.
- Shot: FAIL only.

### EXTRA-SET rows (Ola focus — exploratory; "record behaviour" unless stated; any crash = FAIL)

Before EXTRA-SET-2: `cp ~/.config/ferrite/config.json /workspace/ferrite-qa/logs/config-before-extra-set.json`.

- **EXTRA-SET-1 Open/close paths.** Open Settings via Ctrl+,, ribbon gear, command palette ("Settings"); close via Esc, `×`, and by switching tab. Expected: every path works; no duplicate Settings tabs; no crash.
- **EXTRA-SET-2 Appearance persistence.** Set every Appearance entry to a non-default value (theme, accent, Default View Mode=Split, Font, Rendered view font, Font Size 20, Syntax Theme, Additional Scripts toggles, CJK preference; leave Language for SET-7, title bar covered by TTL-6) → note values → quit cleanly (Alt+F4) → relaunch → all values identical; `python3 -m json.tool ~/.config/ferrite/config.json` reflects them. Shot of before/after recommended.
- **EXTRA-SET-3 Editor persistence.** Flip every Editor toggle and change every value (Tab Size, Max Line Width, Paragraph Indentation, Header Spacing, Vim Mode, Spellcheck, Middle-click paste, Strict Line Breaks, Code Folding, Snippets, Code execution …) → restart → all persisted. Verify at least 3 actually take effect in a document (e.g. line numbers hidden, minimap shown, word wrap).
- **EXTRA-SET-4 Files persistence.** Flip Restore session, Show welcome, Quick note workflow, Auto-Save default, Recent Files count (slider 0..20; try 0) → restart → persisted and observable (e.g. Restore session OFF → previous tabs not restored; Recent files 0 → recent list empty, no crash). Leave *Allow multiple instances* OFF (SI-4 covers it).
- **EXTRA-SET-5 Terminal persistence.** Change every Terminal entry (font size 10/32 extremes, font, scrollback 1000/50000, copy-on-select, theme, opacity 0.1, breathing colour, auto-load layout, sound, auto-focus) → restart → persisted; open the terminal panel → changes visible; opacity 0.1 still usable.
- **EXTRA-SET-6 Keyboard persistence.** Rebind 3 commands (one with Ctrl+Alt+Shift, one function key) → restart → bindings persisted and work; Esc during capture cancels; modifier-only press does not bind; reset-all-shortcuts restores defaults.
- **EXTRA-SET-7 Language switch.** Switch UI language through all offered languages (cs, de, en, es, et, ja, nb_NO, pt, ru, zh_Hans) → each: no raw keys, no tofu (CJK/Cyrillic fonts present), chips/labels not clipped badly → restart in a non-English language → persisted → back to English.
- **EXTRA-SET-8 Invalid inputs.** Into every free-text/number field try: empty, `0`, `-5`, `99999`, `abc`, 500-char string, emoji `🙂`, CJK `中文`. Fields: Max Line Width (px/drag value), Paragraph indentation (em), Tab Size, custom font name `NoSuchFont123` (Font / Rendered view font / Terminal Font), Terminal startup command (`;;;`, `$(rm -rf nowhere)` as text only — do **not** open a terminal session with a destructive command; `echo hello` is the positive case), Custom Prompt Patterns (invalid regex `([`), Sound file path (`/nonexistent.wav`), LSP binary path (`/nonexistent`), Code-execution working dir (`/nonexistent/dir`). Expected: values are clamped/rejected/ignored gracefully; no crash; after restart the app still starts.
- **EXTRA-SET-9 Corrupt config.** Quit Ferrite. `cp ~/.config/ferrite/config.json /tmp/config.good.json`; (a) `printf '{ not json' > ~/.config/ferrite/config.json` → launch → app starts (defaults or `.bak` restore — record which, and any toast/log line); quit; (b) restore good file, then set a wrong type: `python3 - <<'PY'` edit `font_size` to the string `"abc"` (or any numeric field) → launch → record behaviour; (c) restore `/tmp/config.good.json`. Expected: no crash, no hang, user not left with a broken app.
- **EXTRA-SET-10 Search edge cases.** Queries: `[`, `*`, `\`, `.`, `🙂`, `中文`, 200-char string, leading/trailing spaces, upper-case `ZOOM`. Expected: no crash; sensible "no results" state; case-insensitive.
- **EXTRA-SET-11 Rapid changes / live preview.** Drag Font Size and Terminal opacity back and forth quickly for 10 s; toggle theme dark/light 10× quickly; toggle minimap 10×. Expected: no crash, no stuck state, final value persisted.
- **EXTRA-SET-12 Settings in a small window.** Resize the window to ~500×400 with Settings open (and with the search active). Expected: content scrolls, nothing unreachable, no crash.
- **EXTRA-SET-13 Settings across windows.** Ctrl+Shift+N second window → change theme/font size in window 2 → window 1 reflects it (record) → close window 2 → value persisted after restart.
- **EXTRA-SET-14 Reset All then restart.** Reset All → confirm → restart → defaults still in effect, Recently changed cleared/consistent (record).

Afterwards restore your preferred baseline: Reset All (or copy back `logs/config-before-extra-set.json` with the app closed).

---

## Batch 4 — FOCUS: YouTube / video embeds (row 36, VID-*, EOL-7, PLT-2) + EXTRA-YT rows

Fixture: `/workspace/ferrite-qa/ferrite/test_md/test_video_embeds.md` (sections 1–11 have their own "Expect" text — follow them). Video playback is **primary-window only**.

### T2-36 (row 36) — Embed plays, overlays above player, resize, source update
- Steps: open fixture → Rendered → play embed #1 → open **Ctrl+P** and the command palette over the player → drag the resize handle → **Ctrl+S** → Raw.
- Expected: plays (no **Error 153**); overlays paint above the player; resize grows continuously and keeps size on release; source `{{video … width=… height=…}}` updated. If network blocked → `SKIPPED (network)` with the toast/error text.
- Shot: FAIL only (screenshot recommended for focus area: `T2-36-pass.png` while playing).

### VID-1 … VID-10 (total §5) — run all on the same fixture
| Row | Steps (fixture section) | Expected |
|---|---|---|
| VID-1 | §1 play | thumbnail + play → WebView, no Error 153 |
| VID-2 | §2 youtu.be | works like §1 |
| VID-3 | §3 / §4 | width only → 480×≈270 (16:9); width+height → exactly 320×240 |
| VID-4 | §5 vimeo | **never** a WebView (thumbnail/placeholder or opens system browser). No WebView = PASS |
| VID-5 | §6 bare links | stay normal links, no embed |
| VID-6 | §7 drag-resize | grows continuously, keeps size on release, clamps at minimum, source rewritten |
| VID-7 | §8 overlays: palette (Alt+Space), Ctrl+P, Find/Replace, unsaved-changes dialog | each paints above the player; video not destroyed |
| VID-8 | while playing, pointer over raw pane (Split) / title bar | window drag + raw editing still work |
| VID-9 | Ctrl+Shift+N second window, open fixture there | primary keeps playing; secondary shows thumbnail |
| VID-10 | (Linux) whole batch | build uses webkit2gtk (confirmed at prep: binary links libwebkit2gtk-4.1 — see PLT-2) or degrades to thumbnail; no crash |
Also run fixture §9 (same video ×4, resize window), §10 (scroll with cursor over player, half out of pane, out-and-back within 2 s doesn't restart), §11 (fullscreen fills the **Ferrite window**, ESC restores, wheel controls player) and report them as `VID-§9`, `VID-§10`, `VID-§11` notes under VID-6/7 or as separate rows.
Shot: every FAIL.

### EOL-7 (total §1) — video resize in CRLF doc keeps CRLF
- Doc: `/tmp/crlf-video.md` (prep script)
- Steps: open → Rendered → drag-resize the embed → Ctrl+S → `eol-check.sh /tmp/crlf-video.md crlf | tee logs/EOL-7-check.txt`.
- Expected: CRLF True; the rewritten `{{video … width=… height=…}}` line ends in `0d0a`.

### PLT-2 (total §12, Linux) — release build links WebView deps
- Prep already checked `ldd $FER | grep -E 'webkit|soup|gtk'` (see prep report); operator confirms by VID-1 working. Expected PASS if VID-1 plays.

### EXTRA-YT rows (Ola focus)
- **EXTRA-YT-1 URL format matrix.** Open `/workspace/ferrite-qa/fixtures/yt-formats.md` in Rendered. For each case A1–A14, B1–B5, C1–C9, D1–D8 record: thumbnail? player plays? size? any toast? The file states the expectation derived from code for each case (A* = trusted embed, B* = never a WebView, C* = must not embed/crash, D* = dimension handling: 0/-5/abc ignored, 99999 clamped to 8192, keys case-insensitive). Any crash/hang = FAIL; any WebView for a B*/C* case = FAIL (security). Screenshot each FAIL; one overview screenshot of the page anyway.
- **EXTRA-YT-2 Offline behaviour.** Quit all Ferrite processes first (single-instance uses localhost). Launch Ferrite without network:
  `sudo unshare -n -- sudo -u box env DISPLAY=$DISPLAY HOME=$HOME XAUTHORITY=${XAUTHORITY:-} /workspace/ferrite-qa/run-ferrite.sh --tag EXTRA-YT-2 -- --new-instance /workspace/ferrite-qa/ferrite/test_md/test_video_embeds.md`
  (if `unshare` is not usable → `BLOCKED (cannot simulate offline)`; do not change system network config). Expected: thumbnails fall back to placeholder, play shows a sensible error/toast, no hang, no retry storm in the log, no crash; editing still responsive. Then (b) online again: start normally, play, and while playing note behaviour if the player is scrolled away and back.
- **EXTRA-YT-3 Window resizing.** With 2+ players visible (fixture §9 or `yt-many.md`): resize the window from 1920 wide down to ~600 and back, maximize/restore, toggle Split ↔ Rendered, toggle the sidebar/outline. Expected: players follow their slots, no stale WebView floating over other UI, no crash.
- **EXTRA-YT-4 Many embeds.** Open `/workspace/ferrite-qa/fixtures/yt-many.md` (12 embeds). Scroll top→bottom→top quickly 3×; start 2–3 players. Expected: scrolling stays smooth, memory/CPU acceptable (note `ps -o rss= -p $(pgrep -x ferrite)` before/after), no player doubling, no crash.
- **EXTRA-YT-5 Tab/mode switching while playing.** Play a video → switch to another tab → back; switch Raw → Rendered; close the tab while playing; close the window while playing. Expected: audio stops when the embed is gone (record), no orphan WebView, no crash.
- **EXTRA-YT-6 Live editing of an embed.** In Split, edit the URL of a playing embed char by char (break it, then fix it), change `width=` by typing. Expected: no crash; player updates/re-creates sensibly.
- **EXTRA-YT-7 Overlays + dialogs over many players.** With 2 players playing open Settings, the reload dialog (dirty doc + Ctrl+Shift+R), the Search-in-Files panel. Expected: overlays on top (as VID-7).
- **EXTRA-YT-8 Theme + zoom.** While a player is visible change theme and font size/zoom. Expected: embed placement still correct.

---

## Batch 5 — Tier 2 remainder (rows 30–35, 37–40) — 10 rows

| Row | Doc(s) | Steps (Linux) | Expected | Shot |
|---|---|---|---|---|
| T2-30 | `…/test_md/test_find_arrow_nav.md` | Rendered → Ctrl+F `ferrite` → Enter/F3 each match, Shift+F3 back, Ctrl+F again (query selected, typing replaces). Repeat in Split | scrolls to each match; in Split preview scrolls, raw find unchanged | FAIL |
| T2-31 | `…/test_md/test_html_entities.md` | Rendered | `&amp; &rarr; &reg; &#8594;` → `& → ® →`; `&foo;` literal; `<span>` not executed | FAIL |
| T2-32 | `…/test_md/test_emoji.md` | Raw + Rendered | emoji visible (Noto Color Emoji installed); toolbar icons unchanged | FAIL |
| T2-33 | `…/test_md/test_local_images.md` | Rendered | `assets/`, `./assets/`, bare name, backslash variants render; `https://` image = placeholder, no fetch | FAIL |
| T2-34 | `…/test_md/test_copy_and_split_undo.md` Part A | Rendered: select+copy the bash block → `xclip -selection clipboard -o` | `mkdir -p` intact, no inserted spaces | FAIL |
| T2-35 | any doc (e.g. test_find_arrow_nav.md) | Preview lock padlock ON (or **Insert**) → arrows, type, Raw pane edit → restart | arrows move; typing does nothing in preview; Raw editable; lock persists | FAIL |
| T2-37 | any 2 docs | Ctrl+Shift+N → 2nd window; from a 2nd terminal `$FER /tmp/lf-test.md` (Explorer equivalent) → lands in last-focused window; play video in window 1 → keeps playing; close window 2; quit; relaunch | as described; both windows restored | FAIL |
| T2-38 | `/tmp/lf-test.md` | While Ferrite runs: `$FER /tmp/lf-test.md` from a 2nd terminal; then `cd /tmp && $FER ./lf-test.md` | opens as tab in existing window, window raised, 2nd process exits; **relative path must also open** (stale KNOWN GAP → must PASS) | FAIL |
| T2-39 | any | right-click a tab → New / Close / Copy Path (`xclip -selection clipboard -o`) / Reveal (thunar; none → SKIPPED for that sub-step); drag-reorder tabs; Alt+Z | all work; Alt+Z toggles wrap | FAIL |
| T2-40 | — | Windows title bar row | `SKIPPED (platform)`; Linux sanity is reported under TTL-6 (Batch 3) | — |

---

## Batch 6 — Tier 3 (rows 41–47) + Single instance (§6 SI) — 12 rows

| Row | Doc(s) | Steps | Expected |
|---|---|---|---|
| T3-41 | `…/test_md/test_tables.md` | Rendered; edit a cell | `:-:` / `--:` alignment renders; inline formatting preserved |
| T3-42 | `…/test_md/test_data.csv` | double-click cell → edit → Tab → Ctrl+Z | next cell; undo reverts |
| T3-43 | `…/test_md/test_git_graphs.md`, `…/test_md/test_flowcharts.md` | Rendered | render, no overlapping nodes |
| T3-44 | `…/test_md/test_github_html.md` | Rendered | `<details>` collapses, `<kbd>` renders, `<div align=center>` centred |
| T3-45 | `…/test_md/test_code_execution.md` | Run a Python block (§2c; consent on first run) → edit a line above the fence | output attached; stays attached (python3 is on PATH) |
| T3-46 | new doc with `## 1\.3\.1 Title` (+ some text) | Outline panel → click | shows `1.3.1 Title`; jumps |
| T3-47 | open workspace `/workspace/ferrite-qa/fixtures` (has `sample.pdf`, `sample.zip`) and repo tree `assets/icons/icon-192x192.png`, `portable/FerriteMDPortable/FerriteMDPortable.exe` | open png, pdf from tree; open the `.exe` (or `.zip`) | png/pdf viewer tabs; `.exe`/`.zip` → OS default app via xdg-open + toast (no handler on box → record toast; not a FAIL if the toast appears) |
| SI-1 | `/tmp/lf-test.md` | Ferrite running; 2nd terminal `$FER /tmp/lf-test.md` | tab opens in existing window, raised, 2nd process exits ≲1 s (`time $FER …`) |
| SI-2 | same | `cd /tmp && $FER ./lf-test.md` | opens (not "file not found") |
| SI-3 | — | `$FER --new-instance` (better: `run-ferrite.sh --tag SI-3 -- --new-instance`) → `ls ~/.config/ferrite/` | 2nd independent window/process; `instance.lock`/`instance.port`/`instance.pid` belong to the primary only (`cat instance.pid` = primary PID) |
| SI-4 | — | Settings → Files → *Allow multiple Ferrite windows/instances* ON → launch again → then OFF | 2nd process spawned (no forwarding); turned back OFF |
| SI-5 | — | `kill -9` the primary → launch | starts normally as new primary, no hang |

Shot: every FAIL.

---

## Batch 7 — §6 extras: Search-in-Files offsets, primary selection, image paste — 10 rows

Workspace: launch with `/workspace/ferrite-qa/ferrite`. Create
`/workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md` with two lines:
`搜索测试 中文 search` and `Istanbul İstanbul istanbul` (delete it after SIF-4:
`rm /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md`).

| Row | Steps | Expected |
|---|---|---|
| SIF-1 | Ctrl+Shift+F → `中文` → click | no crash; one result; caret **on** `中文` |
| SIF-2 | query `search`, case-insensitive | match column correct on the CJK line (caret on `search`) |
| SIF-3 | `istanbul` case-insensitive; click each | all 3 variants found; each lands on the right word (no drift after `İ`) |
| SIF-4 | Raw Ctrl+F Whole word `中文`; then Whole word + Case sensitive `é` in a line `éa é` (use `/workspace/ferrite-qa/fixtures/t1-multibyte.md`) | found, no crash; only standalone `é` |
| PRI-1 | Raw: mouse-select a word, wait 1 s, middle-click in a terminal (or `xclip -selection primary -o`) | word pasted |
| PRI-2 | select text in the terminal (or `printf 'PRIMARY-TEXT' \| xclip -selection primary`), middle-click in Raw → **Ctrl+Z** once | inserted at click position; one undo removes all of it |
| PRI-3 | middle-click a tab | closes the tab |
| PRI-4 | Settings → Editor → *Middle-click paste* OFF → repeat PRI-2 → ON again | nothing inserted |
| IMG-X1 | untitled tab (Ctrl+N) → clipboard image (`xclip … -t image/png -i …/sample.png`) → Ctrl+V; then `ls /workspace/ferrite-qa/ferrite/assets/` and `ls ./assets` in the launcher's cwd | toast "save the document first"; **no** new `assets/` in cwd; repo `assets/` only has pre-existing entries (`dictionaries fonts icons linux macos screenshots web` + gif/jpg) (covers IMG-8) |
| IMG-X2 | saved doc (`/workspace/ferrite-qa/fixtures/t1-multibyte.md`) → paste the same image twice quickly | two distinct PNGs in `/workspace/ferrite-qa/fixtures/assets/`, nothing overwritten |

---

## Batch 8 — §6 extras: Spellcheck + fonts — 12 rows

| Row | Doc / steps | Expected |
|---|---|---|
| SPL-1 | Settings → Editor → Spellcheck default → ON | OFF by default; "Loading dictionary…" briefly in status bar |
| SPL-2 | new doc Raw: `This is a speling mistaek in Ferrite.` | `speling`, `mistaek` squiggled; `This`, `is` not (`Ferrite` may be) |
| SPL-3 | right-click `speling` → `spelling` → Ctrl+Z once | replaced; one undo restores |
| SPL-4 | right-click `mistaek` → Add word → restart → `cat ~/.config/ferrite/spellcheck/user-words.txt` | squiggle gone immediately; stays gone after restart; file contains `mistaek` |
| SPL-5 | type a fenced code block, `` `code` ``, `https://exmaple.com/pth`, `[ok](http://speling.com)`, `camelCaseWord`, `ALLCAPS`, `abc123` | no squiggles in any of them |
| SPL-6 | Rendered and Split | no squiggles in rendered/preview pane |
| SPL-7 | `/workspace/ferrite-qa/ferrite/test_md/test_scroll_perf.md` | smooth scrolling; squiggles only near viewport |
| SPL-8 | Spellcheck OFF | squiggles vanish immediately, no crash |
| FNT-1 | Settings → Appearance → Rendered view font: default *Same as editor* → untick → other built-in face | Rendered changes font, Raw does not |
| FNT-2 | custom system font e.g. `DejaVu Serif` (check `fc-list \| grep -i dejavu`; if absent pick any `fc-list` serif) → restart | applies; persists |
| FNT-3 | Settings → Terminal → Terminal Font (default JetBrains Mono) → system monospace (e.g. DejaVu Sans Mono / Noto Mono) → open terminal panel | glyphs change; cells aligned; no clipped descenders |
| FNT-4 | (optional) without a Nerd Font: in Ferrite terminal `printf '\ue0b0 \uf015 \ue62b\n'` | tofu expected, no toast, no crash. Installing a Nerd Font = optional (needs download; skip → `SKIPPED (optional)`) |

---

## Batches 9–12 — Total test rows NOT already covered (run after Batch 8)

Rows covered by earlier batches (report them in the total-test table as
"covered by T1-n/…" with the same result, do not re-run):
STAT-1/2→T1-12 · EOL-1→T1-1 · EOL-2→T1-2 · EOL-4→T1-3 · EOL-6, EOL-8→T1-4 ·
EOL-7→Batch 4 · LOPS-1..3→T1-13 · SES-2→T1-10 · SES-7→T1-11 · ENT-1..5→T2-31 ·
FIND-1..4→T2-30 · ARR-3→T2-35 · ARR-6 (first half)→T1-17 · CPY-1→T2-34 ·
UND-1/2→T1-19 · RLD-1→T1-5 · RLD-2→T1-6 · RLD-6→T2-27 · RLD-8→T1-7 ·
IND-6/10→T1-20 · EMO-1..3→T2-32 · IMG-1/6→T2-33 · IMG-3/9→T1-14 · IMG-8→IMG-X1 ·
MMD-8→T1-22 · MMD-12→T1-21 · MMD-5/7→T3-43 · VID-*→Batch 4 · MW-1..4→T2-37 ·
MW-6→T2-38/SI-1 · HTML-1..3→T3-44 · TBL-1/4→T3-41 · CSV-1..3→T3-42 ·
UX-1→T2-35 · UX-2/3→T2-39 · UX-5→T3-46 · UX-6→T2-27 · UX-10→T3-47 ·
RUN-3→T3-45 · TTL-6→Batch 3 · PLT-2→Batch 4.

Platform skips (brief §5): TTL-1…TTL-5, MAC-1…MAC-4, PLT-1, PLT-3 →
`SKIPPED (platform)`; RUN-2 `(Windows)` → `SKIPPED (platform)` (brief: on Linux a
`bash` fence should simply run — check §2d of the code-execution fixture once and
note it); GIT-1 (Windows `\\?\` verbatim path) → `SKIPPED (platform)`;
STAT-4 (German dead key) → optional (`SKIPPED (optional)` if no layout switching).

### Batch 9 — §1 data integrity + §2 rendered fidelity remainder (15 rows)
| Row | Doc | Steps (Linux) | Expected |
|---|---|---|---|
| STAT-3 | new doc | `- item`, `* item`, `1. item` | list detection / stats unchanged |
| EOL-3 | `/tmp/crlf-test.md` (reset: `prep-fixtures.sh eol`) | Split: edit in preview pane → save → `eol-check.sh … crlf` | CRLF preserved |
| EOL-5 | `/tmp/lf-test.md` | Rendered edit → save → `eol-check.sh /tmp/lf-test.md lf` | LF-only |
| EOL-9 | `/tmp/mixed-eol.md` | edit + save → `file`, `eol-check.sh … crlf` | one consistent (majority = CRLF) ending, no crash |
| SES-1 | 2 modified saved tabs + 1 modified **untitled** tab | Alt+F4 → Save | every tab saves (untitled via Save As file dialog → save to `/workspace/ferrite-qa/fixtures/untitled-save.md`); dialog completes |
| SES-3 | 2 windows | close window 2 via its `×` → Save | saves that window's tabs then closes |
| SES-4 | modified tab | close tab → Don't Save; also window close → Don't Save; inspect `~/.config/ferrite/` for autosave/recovery files (`find ~/.config/ferrite -newer /tmp/lf-test.md -type f`) | autosave + recovery files removed |
| SES-5 | untitled quick note | type text → quit → relaunch | content kept across restart |
| SES-6 | 2 windows, each with a dirty tab | close each → confirmation names the right tab | correct tab targeted |
| ENT-6 | `test_html_entities.md` | Split preview | same as Rendered |
| FIND-5 | `test_find_arrow_nav.md` | Raw find next/prev | unchanged behaviour |
| ARR-1/2/4/5 | `test_find_arrow_nav.md` | Rendered arrows between blocks; inside multi-line edit by visual row; Split raw vs preview focus; Raw-only arrows | as total test §2 |
| ARR-6 (2nd half) | new doc `## #1 issue` | Rendered edit round-trip | heading survives |
| CPY-2/3 | `test_copy_and_split_undo.md` Part A | copy rust line + inline-formatted paragraph → `xclip -selection clipboard -o` | no inserted spaces |
| UND-3 | `test_copy_and_split_undo.md` Part B | same as T1-19 in Rendered (non-split) | clean undo |

### Batch 10 — §3 editor QoL remainder (14 rows)
| Row | Doc | Steps | Expected |
|---|---|---|---|
| RLD-3 | dirty tab | Reload → Save path | works as designed |
| RLD-4 | untitled tab | Ctrl+Shift+R | toast "nothing to reload", no crash |
| RLD-5 | any | ribbon Save dropdown → Reload | works |
| RLD-7 | `/tmp/crlf-utf16.md` and `/tmp/cp1252.md` (reset with `prep-fixtures.sh eol`) | open → external edit (`printf 'X\r\n' \| iconv -f UTF-8 -t UTF-16LE >> /tmp/crlf-utf16.md`; `printf 'Caf\xe9 2\n' >> /tmp/cp1252.md`) → Ctrl+Shift+R → readable → edit+save → `eol-check.sh /tmp/crlf-utf16.md utf16`, `eol-check.sh /tmp/cp1252.md cp1252`, `iconv -f CP1252 -t UTF-8 /tmp/cp1252.md` | no mojibake; cp1252 file still has `e9` (not `c3a9`); utf16 file valid |
| IND-1..5, 7, 8, 9 | `…/test_md/test_indent_selection.md` (blocks 1–5) | per total test §3 IND rows (spaces ON size 4 → 4 spaces; spaces OFF → `\t` (check `cat -A`); empty caret Tab; multi-cursor; Block 5 multibyte; Rendered list Tab) | as listed |
| EMO-4 | `test_emoji.md` | ZWJ family | renders or degrades, no crash |
| EMO-5 | `test_emoji.md` / `test_chinese.md` | CJK next to emoji | CJK correct |
| IMG-2 | saved doc | drag-drop `test_md/assets/sample.png` from thunar into Ferrite (if no DnD source available → `SKIPPED (no drag source)`) | saved under `./assets/`, markdown inserted, renders |
| IMG-4 | after a paste | Ctrl+Z | removes markdown insert |
| IMG-5 | any | paste plain text; paste an image URL (`printf 'https://example.com/a.png' \| xclip -selection clipboard`) | text pastes; image-URL smart paste unchanged |
| IMG-7 | preview locked | paste image into Raw | allowed; preview mutations gated |

### Batch 11 — §4 Mermaid remainder (9 rows)
Fixtures: `…/test_md/test_mermaid_v031.md`, `…/test_md/test_pos_hints.md`, `…/test_md/test_flowcharts.md`, `…/test_md/test_mermaid_issue_165.md`.
MMD-1 (popup: click / Ctrl+Click, zoom, Hand pan, X close, theme) · MMD-2 (`fa:fa-*` stripped) · MMD-3 (`linkStyle interpolate basis` smooth) · MMD-4 (subgraph title width stable, also in popup zoom) · MMD-6 (`@pos` hints; invalid id → warning, still renders) · MMD-9 (`subgraph 业务客户端 PEP` full title) · MMD-10 (FC-165b autonumber 1–10, alt/else, `autonumber off`, `autonumber 10 10`) · MMD-11 (FC-165a no overlap, `<-->` two arrowheads) · MMD-13 (CJK node labels hug text, identical after scroll away/back). Expected = checklist text.

### Batch 12 — §6–§10 remainder (≈15 rows)
| Row | Doc | Steps | Expected |
|---|---|---|---|
| MW-5 | — | v1 session file: only if a v1 sample exists; otherwise `SKIPPED (no v1 session fixture)` — do not hand-craft beyond the checklist | loads single window, no crash |
| HTML-4/5/6 | `test_github_html.md` | Rendered | `<sup>/<sub>/<br>`; `<img width height>`; single-line `<div align>` coalesces |
| TBL-2 | `test_tables.md` | toolbar alignment cycling → rendered edits → Raw | delimiters round-trip |
| TBL-3 | `test_tables.md` / `alignment_test.md` | Raw column guides | aligned; no source mutation (`git diff --stat` shows nothing unless you saved) |
| CSV-4 | `…/test_md/very_large_test_data.csv` | open, try to edit | prompts to edit in Raw |
| CSV-5 | `test_data.csv` / `test_data.tsv` | long values | clipped inside columns |
| UX-4 | outline panel | — | guides, bullets, pill highlight, tooltips |
| UX-7 | 2+ tabs | ribbon back-to-previous-tab arrow | works |
| UX-8 | mixed-format line | click-to-edit caret | lands where clicked |
| UX-9 | `…/test_md/test_local_links.md` | click file/folder/web links | file → reveals parent in thunar; folder opens; web → browser; ignored schemes ignored (no file manager → `SKIPPED (no file manager)`) |
| UX-11 | — | F1 help + markdown quick reference | list Reload + word wrap shortcuts |
| WS-1 | workspace tree | hover + active row | highlight/emphasis |
| WS-2 | Stats tab | — | runtime modules; LSP row = Disabled |
| RUN-1/4/5 | `test_code_execution.md` §2c, §4, §10, §11 | Python run, Stop, timeout 5 s, "Waiting for output…", stderr copy/insert labelled | as listed |
| GIT-2/3 | workspace = repo | create `test_md/zz_untracked.md` → badge; save/focus refresh; remove file after | new untracked badged; badges refresh; no `\\?\` in UI |

---

## End of session checklist
1. Quit Ferrite cleanly; `pgrep -a ferrite` empty.
2. `rm -f /workspace/ferrite-qa/ferrite/test_md/zz_cjk_search.md /workspace/ferrite-qa/ferrite/test_md/zz_untracked.md`
3. `git -C /workspace/ferrite-qa/ferrite status --short > /workspace/ferrite-qa/logs/git-status-after-qa.txt` (report it; restore fixtures with `git checkout -- test_md` only — never commit).
4. Write the report per `/workspace/ferrite-qa/report-format.md`; attach `shots/` and `logs/` (zip if needed).
