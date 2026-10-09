# Ferrite v0.3.1 — GrokBot QA Round 2 Close-out (PRD)

> **Status:** Formal PRD — folds into the **0.3.1 branch** after `prd-v0.3.1-qa-closeout.md` (round 1, fully implemented by the orchestrator in `51f5a3a`).
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into cyclopsctl. **Not yet parsed — the orchestrator is started manually later.**
> **Source:** `grokbot/New/qa-report-v0.3.1-linux.md` (ISS-1…63, 198 rows, Linux X11 / Xvfb + llvmpipe, 2026-10-08/09, tested at `8e6ef50`) and the strict subset `grokbot/New/t2-3/`. Screenshots in `grokbot/New/*.zip`, logs alongside.
> **Relationship:** ISS-1…29 and ISS-31 were addressed by round 1 (commits `5325e9b`, `97347d3`, `51f5a3a`). Of the new findings, the isolated ones were fixed directly in **`5eec844`** (ISS-35, 37, 43, 45, 46, 53, 55, 57, 58, 62, 63 + ISS-38 dialog wording + an ISS-36 diagnostic net). This PRD covers what is left: one unexplained data-loss sighting, the rendered-edit commit/undo cluster, mermaid layout fidelity, and a tail of minor items plus a re-run gate.

## 1. Overview

Round 2 confirmed that the round-1 blockers were real (the ISS-1 deadlock hit 22 times, ISS-2 GDK abort 9 launches) — both are fixed on HEAD and are **not** part of this PRD except as rows to re-verify. What round 2 adds falls into five themes:

- **ISS-36 — a restored tab's buffer replaced by three typed characters (blocker, seen once).** Not reproducible from code; the restore path was audited and is correct. A warn-level diagnostic now logs the pre-frame selection, click count and input events whenever a buffer shrinks by more than half in one frame. The task is an *investigation with a reproduction budget*, not a blind fix.
- **Rendered-edit commit timing & undo (ISS-40, 41, 52, 39).** Edits made in the rendered view (table cells, paragraphs, code blocks) reach the document late — a save made while a cell still has focus misses the edit; the Raw pane, status bar and dirty marker lag. Code-block typing in Rendered drops characters and undoes per keystroke (round 1 added coalescing to the raw editor; the rendered code-block session still records per frame). Click-to-edit caret mapping is ~10 source chars off on mixed-format lines, and mouse-selection copy from a highlighted code block inserts spaces at token boundaries.
- **Links & HTML in Rendered (ISS-42, 33).** Ctrl+click / middle-click on links inside list items do nothing because the list-item click-to-edit path consumes the click first; two adjacent single-line `<div align>` blocks render the document's *first line* instead of their content.
- **Mermaid fidelity (ISS-32, 59, 60) — decision needed.** Subgraph layout (edges through titles, nested subgraphs flattened, dropped nodes/labels), CJK box widths 1.6–1.7× label width, and `autonumber off` mid-diagram. These are layout-engine work with their own PRD (`prd-v0.3.1-mermaid-fidelity.md`); this PRD only asks for a go/no-go and records the rows.
- **Minor tail (ISS-30, 34, 44, 47, 48, 49, 50, 51, 54, 56, 61) and observations** — colour emoji, raise-on-forward under xfwm4, Turkish dotted-I search, spellcheck misses/squiggle position, Raw list continuation (triage), mixed-EOL normalisation (triage), Shift+Tab on hard tabs, custom title-bar restore on X11, language dropdown vs shipped locales, Raw table guides, plus harness-side observations (SI-3/SI-4/GIT-3, clippy hygiene).

**Theme:** "What you see in Rendered is what gets saved." Every rendered-view edit must be in the buffer before any save, status or view switch reads it — and nothing a user types may ever replace a restored document.

### Issues mapped

| Issue | Severity (QA) | Scope | Tier | Status |
|-------|---------------|-------|------|--------|
| ISS-36 restored tab buffer replaced by typed text | **blocker** (seen once) | §5.1 | A | diagnostic net in `5eec844`; investigate |
| ISS-41 rendered/preview edits reach document late; save misses cell edit | major | §5.2 | A | open |
| ISS-40 Rendered code-block typing dropped/garbled; per-keystroke undo | major | §5.2 | A | open |
| ISS-52 click-to-edit caret ~10 chars off on mixed-format lines | minor | §5.2 | A | open |
| ISS-39 mouse-selection copy from code block inserts spaces (#162) | major | §5.2 | A | open |
| ISS-42 Ctrl/middle-click on links in list items does nothing (#157) | major | §5.3 | A | open |
| ISS-33 adjacent single-line `<div align>` render first document line | major | §5.3 | A | open |
| ISS-32 mermaid subgraph layout defects | major | §5.4 | decision | open |
| ISS-59 mermaid CJK node boxes 1.6–1.7× too wide | minor | §5.4 | decision | open |
| ISS-60 `autonumber off` mid-diagram; badge overlap | minor | §5.4 | decision | open |
| ISS-30 emoji monochrome / blank with Noto Color Emoji | major | §5.5 | B | limitation; document |
| ISS-44 `İstanbul` not matched by `istanbul` | minor | §5.5 | A | open |
| ISS-47 spellcheck misses `mistaek`, flags link text `ok` | minor | §5.5 | A | open |
| ISS-48 squiggles drawn at right edge of column (wrap) | minor | §5.5 | A | open |
| ISS-51 Shift+Tab does not outdent a hard-tab line | minor | §5.5 | A | open |
| ISS-61 Raw table guides follow header only; none with wrap | minor | §5.5 | B | open |
| ISS-49 Raw Enter does not continue lists | minor (triage) | §5.5 | C | decide |
| ISS-50 mixed-EOL file keeps per-line endings | minor (triage) | §5.5 | C | decide |
| ISS-56 language dropdown 5 of 10 locales; English fallbacks | minor | §5.5 | B | open |
| ISS-34 forwarded open does not raise window (xfwm4) | minor | §5.6 | B | verify on Windows |
| ISS-54 custom title bar restore/drag on X11 | minor | §5.6 | B | verify on real display |
| ISS-38 Exit→Save skips Untitled (Quick note workflow) | major→by design | §5.6 | — | dialog explains it (`5eec844`); doc |
| Observations SI-3, SI-4, GIT-3, CSV-4, HTML-4, SPL-3, clippy | — | §5.6 | B/C | open |
| Re-run gate (ISS-1/2/5/7/9 + all `5eec844` rows) | — | §6 task 8 | A | open |

---

## 2. Goals

- An edit made anywhere in the Rendered view (table cell, paragraph, list item, code block) is in `Tab::content` **before** any save, view switch, status-bar refresh or session snapshot reads it. The dirty marker appears on the first keystroke.
- A rendered code-block edit session undoes in **one** step (same rule as the raw editor since round 1) and never drops typed characters.
- Clicking on `tail` in a mixed-format rendered line puts the caret on `tail`.
- Copying a mouse selection from a highlighted code block is byte-identical to the source.
- Ctrl+click / middle-click on a link opens it wherever the link is (paragraph, list item, table cell); a plain click still enters edit mode.
- `<div align>` blocks render their own content regardless of how many are adjacent.
- ISS-36 either has a reproduction and a fix, or a written analysis with the diagnostic log that rules out the restore path and names the remaining hypotheses.
- Case-insensitive search and spellcheck behave correctly for the documented Unicode cases; squiggles sit under their words with word wrap on.
- A documented decision exists for each mermaid row (do in `prd-v0.3.1-mermaid-fidelity.md`, defer, or won't-fix) and for ISS-49 / ISS-50.
- GrokBot re-run of the round-1 and `5eec844` scripts on Linux passes.

## 3. Non-goals

- No mermaid layout-engine work in this PRD (subgraph routing, CJK measurement, autonumber scoping) — §5.4 only produces the decision and hands rows to the mermaid PRD.
- No colour-emoji rasterisation: egui/epaint 0.34 renders glyphs through `ab_glyph` outlines and cannot draw CBDT/COLR bitmap fonts. ISS-30 is documented as a known limitation (monochrome fallback) unless the human opts into a texture-atlas emoji path, which is a separate PRD.
- No new undo UI; the rendered code-block session reuses the round-1 coalescing rules.
- No per-fence code-execution timeout syntax (the fixture's "5 s" is prose, not a feature) — noted as a possible follow-up.
- No changes to Quick note workflow semantics (ISS-38). The exit dialog now says what is kept; the setting tooltip already explains it.
- No fix attempts for ISS-34 / ISS-54 until reproduced outside xfwm4/Xvfb.

---

## 4. Current state (verified in code, 2026-10-09, HEAD `5eec844`)

### 4.1 Session restore & ISS-36 (`src/state.rs`, `src/editor/widget.rs`)
- Restore runs twice: legacy `restore_session_tabs()` from `settings.last_open_tabs`, then `restore_from_session_result()` which clears `self.tabs` and rebuilds from `session.json`. Runtime tab ids keep counting, so they differ from the session's ids; recovery files are keyed by runtime id and the "Ignoring stale recovery file" branch falls through to `ResolvedContent::FromDisk`. Both paths were read end to end — no path produces an empty buffer for an existing file.
- Evidence from the SES-6 log + screenshot: tab 71 (`/tmp/ses1-a.md`) had its content when the editor was created (two heading fold regions were synced), the recovery disk-anchor hash matched none of the trivial strings, `End` produced no caret move, three typed chars at (0,0) left the buffer `" W1"`, Enter made it `" W1\n"` ("1 words, 4 chars, 2 lines"). The only mechanism consistent with every observation is a **whole-buffer selection active at typing time** (or the key events reaching the editor through a non-standard route). Candidates: a stale selection restored with the session (`Tab::selection` / cursor restore vs. `FerriteEditor` selection sync), triple-click `click_count` state carried across tabs, or the `std::mem::replace(editor, FerriteEditor::new())` placeholder in `EditorWidget::show` being observed re-entrantly.
- `5eec844` adds: a `log::warn!` when a buffer > 64 chars shrinks by more than half in one frame (pre-frame selection anchor/head, `click_count`, summarised input events), and `cleanup_ferrite_editor` now also drops `content_hashes/lengths/versions` for the tab id.

### 4.2 Rendered edit sessions (`src/markdown/rendered_session.rs`, `src/markdown/editor.rs`, `src/markdown/widgets.rs`)
- Table-cell and paragraph edit sessions keep their text in a per-block egui `TextEdit` and write back to `Tab::content` on **commit** (focus loss, click on another cell, Done). `flush_all_rendered_sessions(ctx)` exists and is called from `handle_close_request` and the Save path in some places, but `handle_save_file` from Raw (after a view switch) and the status bar / dirty marker read `Tab::content` directly — hence TBL-2: Raw shows the old cell, the save writes the old cell, `*` appears late.
- Code-block edit session (`UND-3`): records `Content modified in split rendered pane, recorded for undo` per frame; round-1 coalescing lives in `EditHistory`/raw `record_external_edit_from_snapshot` and was not applied to the rendered code-block commit path, so undo is per keystroke. Dropped/garbled characters coincide with frames where the session's `TextEdit` lost focus (log shows the block leaving edit mode after the first char in pass 1).
- Click-to-edit caret: `table_cell_raw_cursor_at_click` / paragraph equivalents map a display-galley position back to a raw offset via `map_displayed_to_raw`; the mapping accounts for entity spans but not for the hidden markup of **bold/italic/code/link delimiters** earlier on the line, so the raw caret drifts right by the number of hidden delimiter chars (~10 on the UX-8 line).
- Code-block mouse selection copy (`CPY-2`): the highlighted block is laid out as one `LayoutJob` with one section per syntect token; the selection copy path reads text per *section/galley row* and joins with a space. The block's **Copy** button uses the raw source and is correct.

### 4.3 Links and HTML (`src/markdown/widgets.rs`, `src/markdown/editor.rs`, `src/markdown/parser.rs` HTML-block handling)
- `LinkWidget` (widgets.rs ≈5150–5200) implements middle-click → `open_link_target`, Ctrl/Cmd+click → open, plain click → edit popup, with tooltips. In **list items and paragraphs** the rendered editor's click-to-edit hit test runs first and switches the block to source, so the link widget never sees the click (UX-9: no `Opened link target` log line; plain click entered edit mode).
- Single-line `<div align="…">text</div>` is supported (`test_html_single_line_div_align`), but when two such blocks are separated by a blank line the renderer draws **the document's first line** twice (HTML-1/HTML-6). Likely the HTML block's line range is resolved from a node whose `start_line` is 0/unset, and the content slice falls back to line 0. Multi-line `<div align>`, `<details>`, `<kbd>` are fine. Side note: `<strong>` inside `<summary>` shows as tag chips.

### 4.4 Mermaid (`src/markdown/mermaid/flowchart/`, `sequence.rs`)
- Flowchart layout: Sugiyama in `layout/sugiyama.rs`; subgraphs are drawn as frames around member nodes **after** node placement; edge routing does not treat subgraph title bands as obstacles; nested subgraphs are laid out as siblings. `one --> two` where both are subgraph ids creates phantom nodes.
- Node width for CJK: text measured through `EguiTextMeasurer`; the CJK fallback font's advance metrics appear to be over-estimated (1.6–1.7× for pure-CJK labels, 1.0–1.4× for mixed).
- Sequence `autonumber`: only the last top-level directive applies (documented in `docs/technical/mermaid/sequence-autonumber-rendering.md`); the checklist row MMD-10 expects mid-diagram `off`.

### 4.5 Search, spellcheck, editor minors
- Search-in-files (`src/ui/search.rs`) and Find (`src/editor/find_replace.rs`) lower-case both haystack and needle with `str::to_lowercase()`; `İ` (U+0130) lower-cases to `i̇` (two scalars), so `istanbul` never matches. Case folding (`unicase`/`caseless` or a `char::to_lowercase().next()` first-scalar fold) is the usual fix.
- Spellcheck (`src/spellcheck/`): `mistaek` passes because the dictionary's suffix rules accept it or because the word was in `user-words.txt` at the time (GrokBot could not tell); link text `ok` is tokenised as a word — correct per tokenizer, but 2-letter words should probably be skipped or the link text excluded by setting (`tokenize.rs` has `indented_code_vs_list_continuation` style tests to extend). Squiggle x-position is computed from the *unwrapped* line layout, so with word wrap on it lands one wrap-width to the right (SPL-7).
- `IND-2`: outdent (`Shift+Tab`) removes up to `tab_size` leading spaces and does not handle a leading `\t`.
- `TBL-3`: `src/editor/ferrite/table_guides.rs` returns early when `wrap_enabled`; with wrap off the guide x-positions come from the header row only, and the body rows differ in a proportional font.
- `STAT-3`: `split_list_item` (`ast_ops.rs`) is only wired into the Rendered editor; Raw Enter has no list continuation. Whether Raw should have it is a product decision (Notepad++ does not; VS Code/Obsidian do).
- `EOL-9`: `Tab` keeps per-line endings as read and writes them back unchanged; the checklist expected majority normalisation. Decision needed (preserving is defensible; many editors normalise).

### 4.6 Platform & locale
- `Language::all()` lists 5 of the 10 `locales/*.yaml` (cs, et, nb_NO, pt, ru missing from the dropdown). Several Keyboard labels and "Use system title bar" / "Restart Ferrite…" are English-only in the translated files.
- Forwarded open (`single_instance.rs`) calls `request_user_attention(Informational)` and `ViewportCommand::Focus`; xfwm4 focus-stealing prevention downgrades the raise to an attention hint when another Ferrite window is on top.
- Custom title bar on X11: double-click restore flips the icon but the viewport keeps 1920×1200 at x=−232; drag-from-maximised does not restore (winit `drag_window` on a maximised X11 window).
- `--new-instance` second process treats the primary's `session.lock` as stale, removes it on exit and overwrites `session.json`/`config.json` (SI-3 observation). `Allow multiple instances` reaches `config.json` only at a later save event (SI-4). Each save of an untracked workspace file logs `Reloaded externally modified file` right after `Saved file` (GIT-3; own-write detection by mtime window misses).
- Clippy: `suspicious_open_options` at `single_instance.rs:90/137`; deprecated `egui::Context::run` at `widgets.rs` tests.

---

## 5. Functional requirements

### 5.1 Tier A — ISS-36 investigation (headline, time-boxed)

**Step 1 — reproduce (budget: half a session).** Script the exact SES-1→SES-6 sequence on Linux and Windows: open a ≥7-line saved file, quit cleanly, relaunch (both restore paths: `last_open_tabs` legacy and multi-window `session.json`), click into the tab, `End`, type three characters. Run with `--log-level debug` and watch for the new `EditorWidget: buffer for tab N shrank` line. Vary: a second window open; a recovery file present for the same runtime tab id; Vim mode on/off; triple-click before relaunch (so `click_count` is non-zero); word wrap on.

**Step 2 — harden regardless of outcome (always do):**
1. Session restore must never restore a **selection**; only the cursor. Audit `Tab::cursor_position` / `Tab::selection` restore and the first-frame `FerriteEditor` sync (`sync_editor_from_tab` or equivalent) so a restored tab starts with an empty selection and `click_count = 0`.
2. In `EditorWidget::show`, when the buffer shrinks by > 50 % *and* the frame's input is a single `Text`/`Key` event with no selection before the frame, treat it as anomalous: log at **error**, and push the pre-frame buffer into the undo history as its own group so a single Ctrl+Z restores it. (Never block the edit — the user might have selected all deliberately via Ctrl+A in the same frame; check for `Key::A` + command in the events before flagging.)
3. Replace the `std::mem::replace(editor, FerriteEditor::new())` checkout in `EditorWidget::show` with a take/put that cannot be observed by a re-entrant `get_ferrite_editor_mut` on the same tab id in the same frame (e.g. `Option<FerriteEditor>` slot + debug assertion that the slot is `Some` on put-back).

**Step 3 — report.** Write `docs/technical/files/iss-36-session-restore-typing-investigation.md`: evidence, what was tried, outcome. If reproduced: root cause + fix + regression test (restore → type → buffer length ≥ original + 3). If not: the hardening above plus the remaining hypotheses, and the QA row stays open with "needs a second sighting".

### 5.2 Tier A — Rendered-edit commit timing, undo and caret (ISS-41, 40, 52, 39)

**ISS-41 (implement).**
- Introduce a single `commit_pending_rendered_edits(ctx)` on `FerriteApp` that flushes every live rendered edit session (table cell, paragraph, list item, code block) into `Tab::content`, and call it from: `handle_save_file` / `handle_save_as_file` / `handle_save_all_modified_tabs`, view-mode switch (Raw/Rendered/Split), tab switch, window close/exit (`handle_close_request` already does), session snapshot (`capture_session_state`, `save_recovery_content`), reload-from-disk prompt, and the status-bar word/line count refresh.
- Dirty marker: the first keystroke in any rendered session marks the tab modified (`Tab::is_modified()` must consider pending session text, or the session writes through on every keystroke with a cheap debounce). Prefer **write-through per frame** for table cells and paragraphs (they already serialise cheaply); keep code blocks on commit but mark dirty immediately.
- Split view: the Raw pane shows the edit within one frame of typing.
- Tests: table-cell edit → `handle_save_file` without blur → file contains the edit; paragraph edit → `has_unsaved_changes()` true after first char; snapshot during cell edit contains the edit.

**ISS-40 (implement).**
- The rendered code-block session records **one** undo group per session (focus-in → Done/blur), using the round-1 `EditHistory` coalescing API (`break_group` at session start and end). The per-frame `Content modified in split rendered pane` log becomes one line at commit.
- Dropped characters: the session's `TextEdit` must keep focus for the whole session; identify why the block left edit mode after the first keystroke in UND-3 pass 1 (likely the parent's click-to-edit / `request_focus` racing with the `TextEdit` id across re-layout) and pin the widget id to the block's stable `node id + line range`. Test with egui's `Context::run` harness: type 10 chars in a code-block session → all 10 present, one undo group.

**ISS-52 (implement).**
- `map_displayed_to_raw` (and the paragraph/list equivalents) must account for hidden inline delimiters (`**`, `*`, `_`, `~~`, `` ` ``, link `[`/`](url)`, image syntax, HTML entities) earlier on the same line, not only entity spans. Build the display↔raw index table once per line from the same inline parser used for layout (`parse_inline_markdown` now exposes `InlineDisplaySpan`; extend it with raw byte ranges). Test: `**b** *i* `c` [l](u) text tail` — clicking at display index of `tail` yields the raw index of `tail`.

**ISS-39 (implement).**
- The code-block selection copy must read from the block's **raw source string** using the galley's selected char range, not from per-section/row text joined with spaces. Locate the join (grep for `.join(" ")` / `push(' ')` in the code-block selection path in `widgets.rs`/`editor.rs`) and replace with a slice of the raw text by `char_range` → byte range. Test: select `tems.iter().m` → clipboard `tems.iter().m`. Keep the **Copy** button path as is.

### 5.3 Tier A — Links and HTML in Rendered (ISS-42, 33)

**ISS-42 (implement).**
- Hit-test order: when the pointer is over a link rect inside a paragraph/list item/table cell, **modifier-click and middle-click** are handled by the link (open target) and must **not** fall through to click-to-edit; a plain click keeps the current behaviour (edit). Implement by checking link rects in the block's pre-pass (the rects are known after layout) before the block-level `interact`, or by giving the link a higher-order `ui.interact` registered first. Log `Opened link target via …` on success; on failure show a toast (`open_link_target` error) instead of only `log::error!`.
- Covers file links (reveal in file manager), folder links (open), web links (browser), and `[[wikilinks]]` (open the target tab) consistently in all three block types.
- Test: unit-level on the dispatch (`resolve_openable_link_target` already tested) plus an egui harness test that a Ctrl+click on a link in a list item does not toggle the item into source mode.

**ISS-33 (implement).**
- Reproduce with the fixture `test_md/test_github_html.md` "Adjacent divs". Find why the second and first single-line `<div align>` render `A# GitHub HTML Subset Test`: inspect how the HTML-block path (`parser.rs` node → `widgets.rs`/`editor.rs` renderer) slices the content for an `HtmlBlock` node when two blocks are adjacent (likely `start_line == 0` or an off-by-one in a `lines()` index when the previous node is also an HTML block). Fix the slice; add a parser/renderer test with two adjacent single-line divs separated by a blank line and one with no blank line.
- Side note (low priority, same task): `<strong>` inside `<summary>` renders as tag chips — allow the inline subset inside `<summary>`.

### 5.4 Decision — Mermaid fidelity rows (ISS-32, 59, 60)

This PRD does **not** implement these. The task is to:
1. Reconcile with `docs/ai-workflow/prds/prd-v0.3.1-mermaid-fidelity.md`: for each of ISS-32's sub-findings (edges through titles, nested subgraphs flattened, `direction LR` in subgraph ignored, subgraph-to-subgraph edges creating phantom nodes, dropped node/edge/label in "Asymmetric Shape with Dash-Style Edge Labels", frame overlap FC-165a), ISS-59 (CJK width) and ISS-60 (autonumber scope + badge overlap): mark **in mermaid PRD / defer to 0.3.2 / won't fix**, with one line of reasoning.
2. For ISS-60 specifically: either implement mid-diagram `autonumber off` (small: track a per-message `numbered` flag while parsing) **or** update checklist row MMD-10 to match `sequence-autonumber-rendering.md`. The badge/label overlap is a rendering offset fix and may be done here if cheap.
3. For ISS-59: measure one CJK label with `EguiTextMeasurer` vs. the painted galley width and record the ratio in the mermaid PRD; if the measurer uses a different font family than the painter, that is a one-line fix and should be done here.
4. Output: a short section appended to `prd-v0.3.1-mermaid-fidelity.md` and updated Status cells in the QA report rows.

### 5.5 Tier A/B/C — Search, spellcheck, editor and locale minors

- **ISS-44 (A):** case-insensitive compare uses a simple fold (first scalar of `char::to_lowercase()`, or the `unicase` crate if already a dependency) in Find, Find-in-files and Replace. Test: `İstanbul Istanbul istanbul` + query `istanbul` → 3 hits with correct byte offsets; `ß`/`SS` left as-is (document).
- **ISS-47 (A):** (a) skip words shorter than 3 chars by default (`ok`, `an`) — setting-free constant; (b) add a tokenizer test that `[ok](http://speling.com)` yields only `speling` *if* "ignore link text" is the intended rule, otherwise document that link text is checked; (c) check `mistaek` against the bundled dictionary offline and fix the suffix rule or the word list if it is accepted.
- **ISS-48 (A):** squiggle x/y computed from the **wrapped** galley (row + x within row) — reuse the cursor-to-screen mapping the caret uses. Test with a forced wrap width that `viewport` on a wrapped line gets a squiggle under the word.
- **ISS-51 (A):** outdent removes one leading `\t` when present, else up to `tab_size` spaces; multi-line selection applies per line. Test on the IND-2 fixture lines.
- **ISS-61 (B):** guides per **row**: compute pipe x-positions for each visible table row from its own galley and draw per-row segments (or only draw guides when the font is monospace). Keep the wrap-on early return but say so in the tooltip/status. Low priority.
- **ISS-49 (C, decide):** human decision whether Raw Enter continues `-`, `*`, `1.` lists (and `- [ ]`). If yes: reuse `split_list_item` on Raw Enter with the same escape (Enter on an empty item ends the list), setting `editor.list_continuation` default on. If no: close the checklist row with a note.
- **ISS-50 (C, decide):** human decision whether mixed-EOL files are normalised to the majority ending on save. If yes: `Tab::detect_line_ending` majority + normalise on save with a status-bar indicator; if no: close the row and document "endings preserved per line".
- **ISS-56 (B):** `Language::all()` lists every locale with a `locales/*.yaml` (cs, de, en, es, et, ja, nb_NO, pt, ru, zh_Hans); fill the English-only keys reported in ja/zh_Hans (`settings.*.use_system_title_bar`, restart notice, Keyboard labels for Toggle Word Wrap, Lock editing, Toggle Terminal, Toggle Productivity Hub, Zoom In, Reset Zoom). Existing `locale_keys_exist_in_all_yaml` test extended to the keyboard labels.
- **ISS-30 (B, document):** write `docs/technical/platform/emoji-rendering-limitation.md`: epaint cannot rasterise colour bitmap fonts, so Noto Color Emoji / Segoe UI Emoji render monochrome via the outline fallback and some codepoints (variation selectors, ZWJ sequences, regional indicators) have no outline glyph. Make sure the two blank positions on `test_emoji.md` line 9 at least show the monochrome glyph (check the pre-warm list and the `Failed to find replacement characters` WARN — register U+FE0F as zero-width). Link from `docs/index.md` and the README known-issues.

### 5.6 Tier B/C — Platform verification and observations

- **ISS-34 (B):** verify on Windows and on a Linux WM without focus-stealing prevention. If it reproduces: after `switch_to_tab` also `ViewportCommand::Focus` on the *owning* viewport id (secondary windows have their own ids) and, on X11, set `_NET_ACTIVE_WINDOW` via winit's `focus_window()`. Otherwise close as WM behaviour.
- **ISS-54 (B):** verify on a real X11 display with a compositor. If it reproduces: on title-bar double-click restore, send `ViewportCommand::Maximized(false)` *and* re-apply the stored pre-maximise `OuterPosition`/`InnerSize`; on drag-from-maximised, un-maximise first and start `drag_window` on the next frame.
- **Observations (C, small, same task):**
  - SI-3: a `--new-instance` process must not treat the primary's `session.lock` as stale while the primary PID is alive, and must not overwrite `session.json`/`config.json` on exit (write a per-instance session file or skip session save for secondary instances).
  - SI-4: the "Allow multiple instances" toggle (and any Settings toggle) persists on change, not only at the next unrelated save — check the settings-changed → `save_settings()` debounce path.
  - GIT-3: own-save detection for the file watcher: record the (path, mtime, len) written by `save_tab_by_id` and ignore the next change event that matches it, instead of the current time window.
  - CSV-4: with the grid visible for a large file, do not also show the "Showing raw content" banner.
  - HTML-4: heading `## Phase 1 - <br> line breaks` shows a box glyph; `<kbd>` text dropped in headings — inline HTML subset inside headings.
  - SPL-3: the `[hint/spell] Unknown word` tooltip must not render over an open context menu (suppress hover tooltips while any popup is open).
  - ISS-38 doc: add a "Quick note workflow and exit" paragraph to `docs/technical/files/session-persistence.md` describing the new dialog line.
  - Clippy: fix `suspicious_open_options` in `single_instance.rs` (add `.truncate(..)`), replace deprecated `egui::Context::run` in `widgets.rs` tests with `run_ui`/the harness.

---

## 6. Task breakdown for the orchestrator

Suggested parse into **8 parent tasks** (lowest id first; each independently testable):

1. **ISS-36 investigation + restore hardening** — §5.1. Files: `src/state.rs` (restore paths, selection restore), `src/editor/widget.rs` (checkout slot, anomaly handling), `src/editor/ferrite/editor.rs` (click_count reset), `docs/technical/files/iss-36-session-restore-typing-investigation.md`, `docs/index.md`.
2. **Rendered-edit commit timing & dirty marker** — §5.2 ISS-41. Files: `src/markdown/rendered_session.rs`, `src/markdown/editor.rs`, `src/app/file_ops.rs`, `src/app/mod.rs` (view switch, snapshot), `src/app/status_bar.rs`, `src/state.rs` (`is_modified` / session capture). Tests in `rendered_session.rs` / `file_ops.rs`.
3. **Rendered code-block session: focus stability + one undo group** — §5.2 ISS-40. Files: `src/markdown/widgets.rs` (code-block edit), `src/markdown/rendered_session.rs`, `src/editor/ferrite/history.rs` (group API reuse). Egui harness test.
4. **Click-to-edit caret mapping + code-block selection copy** — §5.2 ISS-52, ISS-39. Files: `src/markdown/widgets.rs` (`map_displayed_to_raw`, `InlineDisplaySpan` raw ranges, selection copy), `src/markdown/editor.rs`. Tests in `widgets.rs`.
5. **Links in blocks + adjacent `<div align>`** — §5.3 ISS-42, ISS-33. Files: `src/markdown/editor.rs` (block hit-test order), `src/markdown/widgets.rs` (`LinkWidget`, toast on open failure), `src/markdown/parser.rs` (HTML block slice), `src/app/mod.rs` (toast). Tests: html block adjacency, link dispatch.
6. **Search / spellcheck / editor minors** — §5.5 ISS-44, 47, 48, 51, 61, 56, ISS-30 doc. Files: `src/ui/search.rs` and `src/editor/find_replace.rs` (the `to_lowercase()` call sites), `src/editor/ferrite/search.rs`, `src/spellcheck/{mod,tokenize,dictionary}.rs`, `src/editor/ferrite/indent.rs`, `src/editor/ferrite/table_guides.rs`, `src/config/settings.rs` (`Language::all`), `locales/*.yaml`, `docs/technical/platform/emoji-rendering-limitation.md`, `docs/index.md`.
7. **Decisions & mermaid hand-off** — §5.4 ISS-32/59/60 and §5.5 ISS-49/50. Output: appended section in `prd-v0.3.1-mermaid-fidelity.md`, decision notes in this PRD's §8, QA report Status cells. Implementation only for the explicitly "do here" items (autonumber-off parse flag or checklist fix; CJK measurer font mismatch if found).
8. **Platform verification, observations, hygiene, re-run gate** — §5.6 ISS-34/54 verification, SI-3/SI-4/GIT-3/CSV-4/HTML-4/SPL-3 small fixes, ISS-38 doc, clippy items; then hand GrokBot the re-run list: ISS-1, 2, 5, 7, 9 scripts (round 1) plus every row fixed in `5eec844` (T2-SET Ctrl+, / gear with system title bar, SES-1/SES-5 dialog wording, RUN-1 output before Stop, PRI-3 middle-click, IMG-X1/X2 toast, TBL-3/T2-39 Alt+Z, T3-41 bold link in cell, MMD-3 indexed linkStyle, MMD-6 @pos warning, UX-11 guide rows, EXTRA-SET terminal Auto-load OFF). Rows that still fail become new issues, not edits to this PRD.

Each task: run `cargo test --bin ferrite` and `cargo clippy --bin ferrite` (fix new warnings only — the tree is not clippy-clean), rustfmt touched files only (the tree is not fmt-clean), commit per task. Do not touch `current-handover-prompt.md` / `ai-context.md` during implementation (cyclopsctl phase rules).

---

## 7. Follow-ups noted during round 2 (not in scope)

- **Per-fence code-execution timeout** (`RUN-1`: fixture says "5 s"): a ```` ```python timeout=5 ```` info-string option would make the fixtures self-describing. Small, separate task.
- **Native title bar hides Zen toggle and the view-mode segment** — both live only in the custom title bar (ISS-37 added only the Settings gear to the ribbon). Decide whether the ribbon should host them too when `native_window_decorations_enabled()`.
- **Colour emoji via texture atlas** — only if the human wants ISS-30 fixed rather than documented.
- **Linux native file dialog on xdg-desktop-portal boxes** fails outright (GrokBot opened files through single-instance forwarding). Worth a fallback to the rfd GTK backend or a toast.
- **Untracked review scratch files** at repo root (`_assess_*.txt`, `_review_diff*.txt`, `src/_tmp_test.rs`, `assets/.jpg`, timestamped assets) are pre-existing and were left alone.

## 8. Decisions log (fill in during task 7)

| Row | Decision | By | Date | Note |
|-----|----------|----|------|------|
| ISS-32 subgraph routing | | | | |
| ISS-32 nested subgraphs | | | | |
| ISS-32 subgraph↔subgraph edges / `direction` | | | | |
| ISS-59 CJK width | | | | |
| ISS-60 autonumber off | | | | |
| ISS-49 Raw list continuation | | | | |
| ISS-50 mixed-EOL normalisation | | | | |

## 9. Acceptance (ship gate for 0.3.1)

- Tasks 1–6 implemented with tests; `cargo test --bin ferrite` green; no new clippy warnings on touched files.
- ISS-36 has either a fix + regression test or the written investigation with the hardening in place; no `EditorWidget: buffer … shrank` warning appears in a 30-minute manual session on Windows and Linux.
- Every decision row in §8 is filled in.
- GrokBot re-run (task 8 list) on Linux: no FAIL on the re-run rows; Tier 1 of `v0.3.1-minimal-release-test.md` all PASS or BLOCKED-by-input only.
