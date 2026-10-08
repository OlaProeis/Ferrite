# Ferrite v0.3.1 — Final Closeout: Review Blockers + GitHub Issues #175–#186 (PRD)

> **Status:** Formal PRD — source of truth for the **final** implementation queue on `0.3.1-experimental` before the `v0.3.1` tag.
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into **cyclopsctl** tasks (new tag/queue; do not reopen `031-prd.md`'s queue).
> **Relationship:** Follows [`031-prd.md`](031-prd.md) (leftover field bugs — all landed) and the 2026-08-08 audit ([`docs/technical/platform/v0.3.1-assessment.md`](docs/technical/platform/v0.3.1-assessment.md) — all 7 blockers landed). This document covers (1) **new blockers found in the 2026-09-08 pre-tag code review** of the uncommitted working tree and (2) **GitHub issues #175–#186**.
> **Branch:** `0.3.1-experimental` · **Baseline:** `cargo test` 1,958 passed / 0 failed · release profile uses `panic = "abort"` — **every reachable panic is an app-killing crash**.
> **Hard constraints:** Stay local-first (no remote image fetch, no font or dictionary downloads). No new Mermaid epic. Do not re-implement anything listed in §4.

## 1. Overview

The v0.3.1 feature set is complete. A five-area read-only review of the current working tree (app/state/platform, editor/markdown, Mermaid, Settings redesign, plus issue investigation) found a small number of **crash and silent-corruption paths reachable from ordinary editing**, and GitHub has accumulated nine open issues in the #175–#186 range, two of which are already fixed on the branch.

This PRD is the last orchestrator queue before tagging. It is deliberately tiered so the human can tag after **Tier A** if time runs out.

### Pillars

1. **Tier A — Tag blockers (crash / data corruption / broken new UI).** Search-in-Files CJK crash (#178), Mermaid parser panics on half-typed input, smart-paste panic, blockquote click-to-edit corruption, Settings-redesign widget-id bug, UTF-16 EOL misdetection, Reload follow-ups.
2. **Tier B — GitHub issues that fit 0.3.1.** Multi-instance setting + single-instance hardening (#186), local image closeout (#182), rendered-vs-raw fonts (#176), Nerd Font terminal glyphs (#185), Linux middle-click paste (#183), Mermaid inline-fit polish (#165 follow-ups from review).
3. **Tier B (last) — Spellcheck MVP (#184).** `spellbook` engine, bundled `en_US`, off by default, raw editor only (§7). Human opted in on 2026-09-08.

### Issues mapped to this PRD

| Issue | Title | Status on branch | Tier | § |
|-------|-------|------------------|------|---|
| [#175](https://github.com/OlaProeis/Ferrite/issues/175) | Find does not scroll to matches in Rendered | **Done** (`find-in-rendered.md`) | — | 4 |
| [#177](https://github.com/OlaProeis/Ferrite/issues/177) | Tab / Shift+Tab indentation shifting | **Done** (`indent-selection.md`) | — | 4 |
| [#178](https://github.com/OlaProeis/Ferrite/issues/178) | Crash searching Chinese characters across file | **Open — confirmed crash** | **A** | 5.1 |
| [#182](https://github.com/OlaProeis/Ferrite/issues/182) | Local images not displayed at all | Core fixed on branch; closeout items open | **B** | 6.2 |
| [#186](https://github.com/OlaProeis/Ferrite/issues/186) | Setting to allow/deny multiple instances | Open | **B** | 6.1 |
| [#176](https://github.com/OlaProeis/Ferrite/issues/176) | Different fonts for rendered view and raw editor | Open | **B** | 6.3 |
| [#185](https://github.com/OlaProeis/Ferrite/issues/185) | Nerd Font glyphs in integrated terminal | Open | **B** | 6.4 |
| [#183](https://github.com/OlaProeis/Ferrite/issues/183) | Middle-mouse (primary selection) paste | Open | **B** | 6.5 |
| [#184](https://github.com/OlaProeis/Ferrite/issues/184) | Spellchecking for Markdown | Open | **B** (last) | 7 |

(#179–#181 are pull requests, not issues.)

---

## 2. Goals

- Zero known panics reachable from typing, pasting, or searching — including CJK text and half-typed Mermaid.
- Click-to-edit inside blockquotes (`> …`, `> [!NOTE]` callouts) round-trips without changing the source structure.
- The new Settings tab is fully usable (sliders drag, color pickers stay open, text fields keep focus).
- Every #175–#186 issue is either fixed on the branch or has a shipped, documented reason to stay open.

## 3. Non-goals

- Spellcheck beyond the §7 MVP: rendered-view squiggles, non-Latin scripts, grammar, auto-correct, simultaneous multi-dictionary.
- Remote `http(s)` image fetching (#182 remote half; policy unchanged).
- Wayland-native primary selection beyond what `arboard` offers.
- Dense Mermaid parity, LSP epic, GitHub HTML Phase 3, `@pos` drag write-back — all already deferred to v0.3.2.
- Version bump / tag / issue closing — human release steps (Appendix B).

---

## 4. Completed work — context only, generate NO tasks

| Item | Evidence |
|------|----------|
| #175 Find scroll-to-match + Ctrl+F select-all | `docs/technical/editor/find-in-rendered.md`, CHANGELOG Fixed |
| #177 Raw Tab/Shift+Tab block indent, multi-cursor fix | `src/editor/ferrite/indent.rs`, `docs/technical/editor/indent-selection.md` |
| #182 core — image paragraphs/list items route to AST `render_image` instead of `LayoutJob` text | `paragraph_needs_ast_inline_widgets` in `src/markdown/editor.rs` (~4007, 4047, 4776, 4938, 6455); `resolve_local_image_path` in `src/path_utils.rs:326` with tests |
| All 2026-08-08 audit blockers B1–B7 and high items 1, 2, 7, 11 | `v0.3.1-assessment.md` status line; CHANGELOG "Pre-release review fixes" |
| Settings redesign — no setting lost (46 registry entries, 1:1 with old `settings.rs`), no missing `en.yaml` keys | 2026-09-08 review |

---

## 5. Tier A — Tag blockers (must ship)

### 5.1 Search in Files crashes on CJK queries ([#178](https://github.com/OlaProeis/Ferrite/issues/178)) + byte/char offset drift

**Problem.** Ctrl+Shift+F with a query containing Chinese (any multi-byte) characters aborts the app on the first match.

**Root cause.** `src/ui/search.rs:304-315` (`SearchPanel::search`):

```rust
let search_line = if self.case_sensitive { line.to_string() } else { line.to_lowercase() };
let mut start = 0;
while let Some(pos) = search_line[start..].find(&query) {
    let abs_pos = start + pos;
    positions.push((abs_pos, abs_pos + query.len()));
    start = abs_pos + 1;          // ← lands inside a 3-byte char → next `search_line[start..]` panics
}
```

Secondary bugs in the same function:
- `to_lowercase()` can change byte length (`İ` → `i̇`), so offsets computed on `search_line` are misaligned against `line` (garbled highlight, wrong caret).
- `SearchMatch.char_offset` is a **byte** offset (`line_start_offset += line.len() + 1`, `abs_pos`), but the consumer `handle_search_navigation` (`src/app/file_ops.rs:1290-1294`) passes it to `tab.set_transient_highlight` / `tab.set_cursor`, which take **char** positions → caret lands past the match in any document with non-ASCII text before it.
- `content.lines()` strips `\r\n` but the offset adds only `+1` → offsets drift one per line in CRLF files.
- Same latent pattern in `src/editor/find_replace.rs:116` and `:171`: whole-word rejection advances `start = match_start + 1` then `text[start..]` → panics when the search term starts with a multi-byte char (Ctrl+F, Whole word, query `é`, doc `éa`).

**Required behaviour / fix.**
1. In `search.rs`, replace the manual lowercase loop with the same approach `find_replace.rs` already uses: build one `regex::RegexBuilder::new(&regex::escape(&self.query)).case_insensitive(!self.case_sensitive)` outside the file loop and use `re.find_iter(line)` for both the literal and regex modes (byte offsets are then always char-boundary aligned and computed on the original `line`). Keep the existing `use_regex` path.
2. If any manual advance loop remains anywhere, advance by the matched char's `len_utf8()`: `start = match_start + text[match_start..].chars().next().map_or(1, char::len_utf8);` — apply this at `find_replace.rs:116` and `:171`.
3. Convert to char offsets at the boundary: in `search.rs` compute `char_offset` with `line_start_char_offset + line[..match_start].chars().count()` and `match_len = line[match_start..match_end].chars().count()`; track `line_start_char_offset += line.chars().count() + eol_len` where `eol_len` is derived by walking the content with `split_inclusive('\n')` (count `\r\n` as 2). Alternatively keep bytes and convert in `handle_search_navigation` via the existing `super::helpers::byte_to_char_offset(content, byte)` (`src/app/helpers.rs:125`) — pick one, document in the struct field doc comment, and rename the field if it stays bytes.
4. Keep `match_start`/`match_end` (used only for highlight painting in the results list) as byte offsets into `line_content` — that code already uses `floor_char_boundary`.

**Key files.** `src/ui/search.rs` (`search`, `SearchMatch`, result row painting ~600-690), `src/app/file_ops.rs:1255-1310`, `src/editor/find_replace.rs:104-178`, `src/app/helpers.rs`.

**Acceptance criteria.**
1. Unit test: `SearchPanel::search` over an in-memory temp file containing `搜索测试 中文 search 中文` with query `中文` (case-insensitive) → 2 matches, no panic; with `İstanbul` query/text → offsets map onto the original line.
2. Unit test: `FindState` whole-word + case-sensitive with term `é` on `éa é` → 1 match, no panic.
3. Unit test: a CRLF file with a match on line 3 → `char_offset` points at the match in the original content (verify by slicing `content.chars().skip(off).take(len)`).
4. Manual: row 15/16 of `docs/technical/platform/v0.3.1-minimal-release-test.md` pass; clicking a result in a CJK document places the caret **on** the match.
5. `cargo test` green.

**Docs.** New `docs/technical/editor/search-in-files-offsets.md` (byte vs char contract) + CHANGELOG Fixed (#178) + `docs/index.md`.

**Complexity hint:** 3

---

### 5.2 Mermaid flowchart parser panics on mismatched brackets (release abort)

**Problem.** While typing a flowchart in Split view, intermediate text such as `A}x{ --> B` or `subgraph a] [b` kills the app. `catch_unwind` guards are irrelevant: release builds use `panic = "abort"` (`Cargo.toml:152`), and `validate_mermaid_source` → `parse_flowchart` (`src/markdown/mermaid/validation.rs:87`, `mod.rs:443`) runs outside any guard anyway.

**Root cause.** Unordered `find`/`rfind` followed by a slice with no `start < end` check:
- `src/markdown/mermaid/flowchart/parser.rs:337-340` — `rest.find('[')` + `rest.rfind(']')` → `rest[bracket_start + 1..bracket_end]`.
- `parser.rs:950-951` diamond `{…}` (`find('{')` + `rfind('}')`), `:1002-1003`, `:1024-1025`, and hexagon `:938-940` (`find("{{")` + `find("}}")` where `}}` may precede `{{`). The asymmetric-shape branch at `:1039` already guards `start < end` — the others don't.

**Required behaviour / fix.**
1. Add a shared helper in `parser.rs`:
   ```rust
   /// Text strictly between the first `open` and the last `close`, or None when
   /// the closer does not follow the opener (half-typed input).
   fn slice_between<'a>(text: &'a str, open: &str, close: &str) -> Option<(&'a str /*before*/, &'a str /*inner*/)>
   ```
   that returns `None` unless `start + open.len() <= end`. Replace every `find`/`rfind` + slice pair in `parser.rs` (grep `rfind(` and `find(` followed by `[` slicing) with it; on `None` fall through to the next shape / plain-id path exactly as when the brackets are absent.
2. Audit `sequence.rs`, `state.rs`, `class.rs`, `er.rs`, `gantt.rs`, `gitgraph.rs`, `pie.rs`, `mindmap.rs`, `timeline.rs`, `journey.rs` parsers for the same pattern (`rg "rfind\(" src/markdown/mermaid`) and apply the helper.
3. Add a **mangled-input corpus test** in `src/markdown/mermaid/mod.rs` (`#[cfg(test)]`): for every fixture in `test_md/test_mermaid_*.md` and `test_md/test_flowcharts.md`, extract each mermaid fence and call `validate_mermaid_source` + the relevant `parse_*` on (a) every prefix of the source (truncate at each newline), (b) the source with every `[`, `]`, `{`, `}`, `(`, `)`, `|`, `"` individually deleted, (c) the source with bracket pairs swapped. Assert no panic (the test simply must not abort; wrap in `std::panic::catch_unwind` in the test only, count failures, assert 0).
4. Optional but recommended: wrap the whole `render_mermaid_diagram` body once with `catch_unwind` for debug builds and delete the misleading per-branch guards; add a comment that release relies on parser panic-freedom.

**Key files.** `src/markdown/mermaid/flowchart/parser.rs`, `src/markdown/mermaid/*.rs` parsers, `src/markdown/mermaid/mod.rs` (tests), `src/markdown/mermaid/validation.rs`.

**Acceptance criteria.**
1. Unit tests: `flowchart TD\nsubgraph a] [b` and `flowchart TD\nA}x{ --> B` and `flowchart TD\nA}}x{{ --> B` parse (as plain nodes / validation warnings), no panic.
2. Corpus test above passes with 0 panics.
3. Manual: minimal-test row 21 (type a fence character by character in Split) — app never closes.
4. Existing 183+ mermaid tests still pass.

**Docs.** Add "Panic-freedom contract" paragraph to `docs/technical/mermaid/mermaid-inline-validation.md` + CHANGELOG Fixed.

**Complexity hint:** 3

---

### 5.3 Smart-paste "URL over selection" panics on multi-byte context

**Problem.** Select a word, Ctrl+V with a URL on the clipboard (smart-paste wraps it as `[word](url)`). If any multi-byte character sits within ±20 **bytes** of the selection, the app aborts.

**Root cause.** `src/app/input_handling.rs:344-347`:
```rust
let search_start = cursor_byte_pos.saturating_sub(sel_bytes + 20);
let search_end = (cursor_byte_pos + sel_bytes + 20).min(content.len());
let search_region = &content[search_start..search_end]; // not char-boundary aligned
```
(The 2026-08-08 fix B3 corrected the cursor math right above this; the slice was missed.)

**Required behaviour / fix.**
1. Snap both bounds to char boundaries before slicing:
   ```rust
   let mut search_start = …; while !content.is_char_boundary(search_start) { search_start -= 1; }
   let mut search_end = …;   while search_end < content.len() && !content.is_char_boundary(search_end) { search_end += 1; }
   ```
   or use `content.floor_char_boundary` / `ceil_char_boundary` if MSRV allows (stable since 1.91? — verify; otherwise use the loops).
2. Preferred simpler alternative: the selection range is already known as char positions (`tab.cursors.primary()`); convert anchor/head to bytes with `char_indices().nth()` and replace exactly that range instead of searching a window.
3. Same file `:227` and `:603`: `let content = tab.content.clone();` runs **every frame** before any event is inspected (full document copy at 60 fps; 80 MB file → 80 MB memcpy per frame). Gate the clone behind "a `Paste` event / Ctrl+V / bracket key exists in `ctx.input().events` this frame".

**Key files.** `src/app/input_handling.rs` (`consume_smart_paste`, `handle_auto_close_pre_render`).

**Acceptance criteria.**
1. Unit test: content `日本語日本語日本語日本語 hello world`, select `hello`, smart-paste `https://x.y` → `[hello](https://x.y)`, no panic; same with selection immediately after an emoji.
2. `cargo test` green; manual row added to minimal test (Tier 1).
3. Frame profiling on a 10 MB file with no paste events shows no per-frame content clone (log-level debug counter or reasoning in the PR description is acceptable).

**Docs.** CHANGELOG Fixed; update `docs/technical/markdown/smart-paste.md` (byte/char note).

**Complexity hint:** 2

---

### 5.4 Rendered click-to-edit corrupts blockquote content

**Problem.** Editing a heading or paragraph that lives inside a blockquote (`> ## Title`, `> text`, and therefore every `> [!NOTE]` callout body) in Rendered/Split rewrites the source wrongly on commit:
- Heading: `> ## Sub` + type one char → source becomes `# Subx` (quote prefix lost, level reset to H1).
- Paragraph: `> quoted one` + type one char → `> > quoted onex`; every further commit adds another `> `.

**Root cause.**
- `src/markdown/editor.rs:2883-2889` (`write_session_block_to_source`, Heading arm): `heading_level_from_source` (`:2818`) counts `#` from column 0 → 0 → clamped to H1; `update_source_line(source, line, &format_heading(&state.text, level))` overwrites the whole line without the `> ` container prefix.
- Paragraph arm `:2892-2901` → `update_source_range` (`:7843-7853`) re-prepends `extract_line_prefix(original_first_line)` (which recognises `> `) to content that was **seeded with the prefix already** (`extract_paragraph_content` joins raw source lines; pinned by test `seed_session_block_text_blockquote_paragraph_not_container_span` asserting `state.text == "> quoted one"`).
- Related medium: in-frame commits (`:2893-2900`, `:2905-2912`) resolve the old block span from the per-frame `edit_state`, which only contains **rendered (non-culled)** nodes; if the dirty block scrolled out of the 500 px overscan, `end_line` falls back to `line` and a shrunk multi-line paragraph keeps its stale trailing lines (duplicated text). The flush path already seeds from the AST (`seed_edit_state_for_active_block`, `:3064`).

**Required behaviour / fix.**
1. Introduce one canonical "container prefix" model: `fn split_container_prefix(line: &str) -> (&str /*prefix: indent + `>`s + list marker*/, &str /*content*/)` in `src/markdown/editor.rs` (or `ast_ops.rs`), unit-tested for `> `, `> > `, `  - `, `> - [ ] `, `1. `.
2. **Seed without the prefix**: `extract_paragraph_content` / `seed_session_block_text` / the TextEdit `source_text` at `:5086` must strip the container prefix from every line of the block (keep inner text only). Update the test at `:9099` to expect `"quoted one"`.
3. **Commit re-attaches the original prefix**: `update_source_line` for headings must write `format!("{prefix}{}", format_heading(text, level))` where `prefix` comes from the original line and `level` comes from the **Heading AST node** (`find_block_node_for_ref(&doc.root, block, line)` → `MarkdownNodeType::Heading { level }`), falling back to counting `#` *after* the prefix. `update_source_range` keeps its prefix logic (now correct because the seed no longer contains it); continuation lines inside quotes get `> ` (not just indentation) — extend the `idx > 0` branch to reuse the quote part of the prefix.
4. For the culled-active-block case, in `write_session_block_to_source` fall back to `find_block_node_for_ref(...).end_line` (parse via `crate::markdown::cache`) when the `edit_state` lookup misses, mirroring `flush_rendered_edit_session`.
5. Do not bump `source_epoch` on these commits (existing contract).

**Key files.** `src/markdown/editor.rs` (`write_session_block_to_source` ~2875-2915, `heading_level_from_source` 2818, `extract_paragraph_content`, `seed_session_block_text` ~833, `update_source_range` ~7830-7870, `update_source_line`, `extract_line_prefix` 7752), `src/markdown/rendered_session.rs` (formatted seed at ~265/418 if it duplicates the seed logic), tests at ~9085-9130.

**Acceptance criteria.**
1. Unit tests (round-trip through seed → mutate → commit): `> ## Sub` → `> ## Subx`; `> quoted one\n>\n> quoted two` edit first → `> quoted onex\n>\n> quoted two`; `> [!NOTE]\n> body` edit body → `> [!NOTE]\n> bodyx`; nested `> > deep` → `> > deepx`; list inside quote `> - item` → `> - itemx`; plain `- item` and plain paragraph unchanged behaviour.
2. Regression test for the culled case: 3-line paragraph at line 200 of a 400-line doc, shrink to 1 line, commit via `write_session_block_to_source` with an `EditState` that lacks the node → lines 201-202 removed.
3. Manual: minimal-test rows 17/18 + new row "edit inside `> [!NOTE]` callout, Escape, check Raw".
4. All existing `seed_session_block_text_*` / RS-1…RS-7 tests pass (update the one that asserted the prefixed seed).

**Docs.** New `docs/technical/markdown/rendered-edit-container-prefix.md` (seed/commit prefix contract) + `docs/index.md` + CHANGELOG Fixed.

**Complexity hint:** 6

---

### 5.5 Settings redesign — widgets lose state when an entry moves to "Recently changed"

**Problem.** In the new search-first Settings tab, the first change to an Essentials setting moves that entry to the top of "Recently changed" on the **next frame**. egui auto-ids are position-dependent, so every auto-id widget in that entry (and all entries after it) gets a new id: slider drags drop after one tick, the accent `color_edit_button_srgba` popup closes on the first pick, un-salted `TextEdit`s (LSP override path, terminal startup command, prompt patterns, sound file) lose focus after one keystroke. The same happens for any recently-changed entry that is not already at index 0.

**Root cause.** `src/ui/settings/mod.rs:316-340` (`render_entry`) and `:405-459` (overview) render entries in a `Vec` order that changes mid-interaction; no stable id scope per entry.

**Required behaviour / fix.**
1. Wrap every entry render in a stable id scope: `ui.scope_builder(egui::UiBuilder::new().id_salt(("setting", entry.id)), |ui| …)` (egui 0.34: `UiBuilder::id_salt`; this is the documented mechanism for widgets that move in the tree). Apply in `render_entry` so search results, chips, essentials and recents all benefit.
2. Freeze the overview order for the current Settings visit: cache the `recent_ids` snapshot when the overview becomes visible (no query, no chip) and reuse it until the tab is left or a query/chip is applied; keep **persisting** `recently_changed_settings` immediately. Entries then never jump under the pointer while being dragged.
3. `mod.rs:165-167` — the unconditional `consume_key(COMMAND, F)`: skip when `self.key_capture.is_some()` (otherwise Ctrl+F can never be bound as a shortcut) and only consume when no other widget has focus or the search field has focus (so a focused integrated terminal keeps Ctrl+F).
4. `registry.rs:82-88` / `mod.rs:334` — exclude `KEYBOARD_SHORTCUTS_ID` from `note_recently_changed` (rebinding a shortcut otherwise puts the whole ~60-row shortcut editor at the top of the overview).
5. `src/config/settings.rs` `sanitize()` (~2897): `recently_changed_settings.retain(|id| registry::entry_by_id(id).is_some()); truncate(MAX_RECENTLY_CHANGED)`; unify the constant (registry says 8, display shows 6).
6. Low items (do in the same task, all small): clear `key_capture` / `conflict_warning` whenever the keyboard entry is not rendered this frame; `about.info` should not count toward the Appearance chip (`section: Option<SettingsSection>` or skip when `!browsable()`); add `settings.clear_search` locale key for the × tooltip (currently reuses `settings.keyboard.cancel`); remove the duplicate "Keyboard Shortcuts" header under the Keyboard chip; add a `files.show_welcome` registry entry for `show_welcome_on_empty_launch` (`featured: true`); delete the orphaned `settings.keyboard.search_hint` key from all locales.

**Key files.** `src/ui/settings/mod.rs`, `src/ui/settings/registry.rs`, `src/ui/settings/keyboard.rs`, `src/ui/settings/files.rs`, `src/config/settings.rs`, `locales/*.yaml`.

**Acceptance criteria.**
1. Manual: with the overview visible, drag the Font Size slider from 14 to 20 in one motion → follows the pointer the whole way; open the accent color picker, pick three colors → popup stays open; type 5 chars into "Terminal startup command" → all 5 land.
2. Rebind a shortcut → overview does not show the shortcut editor as a recent item.
3. Ctrl+F while capturing a shortcut binds Ctrl+F; Ctrl+F in the integrated terminal is not consumed by Settings.
4. Unit test: `sanitize()` prunes an unknown id and truncates to the max.
5. `cargo test` green; minimal-test rows 23–29 pass.

**Docs.** New `docs/technical/ui/settings-search-redesign.md` (registry model, id-scope rule, recently-changed contract) + `docs/index.md` + CHANGELOG (extend the Settings redesign entry).

**Complexity hint:** 4

---

### 5.6 UTF-16 files get the wrong line ending on load (mixed EOLs on save)

**Problem.** Opening a UTF-16 CRLF file (`0D 00 0A 00`) sets `tab.line_ending = Lf`; Enter / line ops then insert `\n` and every rebuild rejoins with `\n`, producing mixed EOLs written back on save. Only `apply_external_disk_reload` was switched to post-decode detection.

**Root cause.** `src/state.rs:1783`, `:2054`, `:2183` call `LineEnding::detect_from_bytes(&bytes)` on **pre-decode** bytes.

**Fix.** Use `LineEnding::detect_from_content(&content)` after decoding in all three constructors (`Tab::with_file`/load, session restore, `finish_loading`). Add a test: UTF-16LE CRLF bytes → `Crlf`. Also `src/app/line_ops.rs` (low): document that line ops normalize mixed-EOL files to the dominant ending (or splice only the touched range).

**Key files.** `src/state.rs`, `src/app/line_ops.rs` (doc comment only).

**Acceptance.** Unit test above; manual: save a UTF-16 CRLF file in Notepad, open in Ferrite, press Enter in Raw, save → still uniformly CRLF (`Format-Hex`). CHANGELOG Fixed (#174 follow-up). Update `docs/technical/files/line-ending-preservation.md`.

**Complexity hint:** 2

---

### 5.7 Reload from Disk follow-ups (#149)

Four small correctness bugs found in the reload path; one task.

1. **Undo history not reset** — `src/state.rs:2832-2864` (`apply_external_disk_reload`) replaces `content` but keeps `edit_history`; Ctrl+Z after a reload replays stale inverse ops at wrong offsets (silent corruption of the reloaded doc). Fix: record the reload as one undo step (`record_edit(old_content → new_content)` before `mark_saved`) **or** clear `edit_history` + `pending_undo_snapshot`; pick "one undo step" for consistency with recovery restore, test it.
2. **Toasts never visible** — `src/state.rs:5114`, `:5130`, `:6577` call `self.show_toast(msg, 0.0, 3.0)` with `current_time = 0.0` → expires immediately after 3 s of uptime. Fix: return the message to `FerriteApp` (caller has `get_app_time()`) or add `AppState::pending_toast` drained by the app each frame.
3. **Large-file stale rope** — `src/editor/widget.rs:549-551` skips resync when the content length is unchanged for >5 MB files; a reload that keeps byte length leaves the FerriteEditor rope stale and the next keystroke writes the old text back. Fix: store `content_version`/`source_epoch` in `FerriteEditorStorage` and force resync when it changes (bump in `apply_external_disk_reload`).
4. **Terminal focus** — `src/app/keyboard.rs:60`: `Reload` sits outside the `!terminal_has_focus` guard. Move it inside. Also skip reload while `tab.is_loading()` (`src/state.rs:5107-5133`) to avoid a synchronous read of a huge file on the UI thread.

**Acceptance.** Unit tests for (1) and (3); manual: edit → Ctrl+Shift+R → Don't Save → Ctrl+Z restores the pre-reload text as one step; untitled tab Ctrl+Shift+R shows the toast after 10 s uptime. CHANGELOG Fixed; update `docs/technical/files/reload-from-disk.md`.

**Complexity hint:** 3

---

## 6. Tier B — GitHub issues and review follow-ups (should ship)

### 6.1 Multiple-instance setting + single-instance hardening ([#186](https://github.com/OlaProeis/Ferrite/issues/186))

**Problem (reporter).** "Open with Ferrite" on each file opens a new instance; wants a setting like VS Code.

**Investigation.** Single-instance is unconditional (`src/main.rs:227` → `single_instance::try_acquire_instance`). Three defects explain the reporter's symptom and related failures:
1. **Startup race** — `try_acquire_instance` (`src/single_instance.rs:45-67`) is read-lock → try-connect → create-listener → write-lock, non-atomic. Selecting N files and choosing "Open with" spawns N processes near-simultaneously; all see no lock and all become primaries → N windows (exactly the report).
2. **No acknowledgement** — `try_forward_paths` (`:185-216`) returns `true` as soon as `write_all` succeeds. A stale `instance.lock` whose port has been reused by another local service makes the secondary exit **without the file ever opening**.
3. **Relative paths** — the secondary forwards `path.display()` unresolved; `ferrite .\notes.md` from a terminal while an instance runs is resolved against the **primary's** CWD → "Skipping non-existent path".

**Required behaviour.**
- New setting `allow_multiple_instances: bool` (default `false`) in Settings → Files ("Allow multiple Ferrite windows/instances"; tooltip: when off, files open in the running instance). Serde default so old configs load.
- New CLI flag `--new-instance` (clap) to force a fresh process regardless of the setting.
- When multiple instances are allowed (setting **or** flag): skip forwarding, do **not** write lock/pid files, `instance_listener = None`.
- Read the setting **before** the single-instance check: move `load_config()` above `try_acquire_instance` in `main.rs` (it is a small JSON read; keep the "fast exit" goal by not initialising logging/icons before the check). Note: `set_locale` etc. stay where they are.
- **Atomic lock**: hold an exclusive OS lock on `instance.lock` for the primary's lifetime using `std::fs::File::try_lock()` (stable since Rust 1.89; MSRV is 1.92). Secondary: if `try_lock` fails → read port → forward; if the port is not yet written (primary still starting), retry connect for up to ~2 s (50 ms sleep) before giving up and becoming a primary. Primary writes the port into the locked file after `bind`.
- **Handshake**: primary replies `OK\n` after reading the message; secondary waits for it with a 1 s read timeout; no `OK` → treat as no instance (remove stale lock, become primary).
- **Canonicalize before forwarding**: `std::fs::canonicalize(p).or_else(|_| std::env::current_dir().map(|d| d.join(p)))` for each path in the secondary; strip `\\?\` via `path_utils::normalize_path`.
- Log (debug) which branch was taken; keep the `__FOCUS__` protocol.

**Key files.** `src/single_instance.rs`, `src/main.rs` (`Cli`, ordering), `src/config/settings.rs`, `src/ui/settings/files.rs`, `src/ui/settings/registry.rs`, `locales/*.yaml`, `docs/technical/platform/single-instance.md`.

**Acceptance criteria.**
1. Unit tests: (a) lock acquired by process A → `try_lock` in the same test process on a second handle fails (Windows + Unix); (b) `read_message_from_stream` + `OK` reply round-trip; (c) relative path canonicalization helper.
2. Manual (Windows): select 5 `.md` in Explorer → Enter → **one** window with 5 tabs. Setting ON → each open spawns its own window. `ferrite --new-instance` always spawns. `ferrite .\relative.md` from a terminal with Ferrite running opens the file.
3. Write a bogus port into `instance.lock` pointing at a listening but non-Ferrite socket (e.g. a PowerShell `TcpListener`) → Ferrite still opens the file (becomes primary).
4. CHANGELOG Added (#186) + Fixed (race/handshake).

**Complexity hint:** 5

---

### 6.2 Local images closeout ([#182](https://github.com/OlaProeis/Ferrite/issues/182))

**Status.** The reporter's exact symptom ("displays the bang and the alternate text") is the 0.3.0 `LayoutJob` path; it is fixed on the branch (§4). Remaining gaps found in review:

1. **Failed image loads are retried every frame** — `src/markdown/editor.rs:7486-7501` caches only `Loaded`; a missing/corrupt/large-unsupported image is `fs::read` + `image::load_from_memory` + `log::warn!` **60×/s** (UI stall on a big corrupt file, log spam). Fix: cache `Failed { msg, mtime: Option<SystemTime>, at: Instant }` too; retry only when the file's `metadata().modified()` changes or after a 2 s backoff; warn once per path.
2. **Stale texture after the file changes** — cache is keyed by path only; overwriting `assets/x.png` keeps the old texture until restart. Fix: include `mtime`+`len` in `cache_id` (cheap `metadata()` once per frame per image is acceptable; or check at most every 500 ms).
3. **Images inside table cells / headings still render as text** — `LayoutJob` cell rendering has no image path. MVP: in table cells render an image node as its alt text with a small image icon + tooltip showing the URL (no crash, clear affordance); document as a known limitation. (Full inline images in cells → v0.3.2.)
4. **Drag-drop clobbers same-name files** — `src/app/file_ops.rs:1620-1640` (`handle_dropped_image`) uses `fs::copy` to `assets_dir.join(new_filename)`; two same-second drops with equal names overwrite. Fix: route through `unique_asset_dest_path` like the clipboard path and link the actual filename.
5. **Drop mid-rendered-edit orphans the active block** — `insert_asset_image_markdown` (`:1670-1696`) mutates `tab.content` and bumps `source_epoch` without `flush_active_rendered_session` first. Fix: flush before insert (thread `ctx` into `handle_dropped_image`).
6. **Untitled docs write `assets/` into the process CWD** — `assets_dir_for_paths(None, None)` → `PathBuf::from("assets")` (`src/path_utils.rs:262-269`). Fix: for pathless tabs with no workspace, show toast `image_paste.save_document_first` and abort the paste/drop (no filesystem write). Locale key in all locales.
7. Add a note to `docs/technical/markdown/local-image-assets.md` covering the reporter's cases (`![x](image.png)` beside the document, `./image.png`) and reply-ready text for the issue.

**Key files.** `src/markdown/editor.rs` (`render_image`, `ImageLoadResult`), `src/markdown/widgets.rs` (table cell inline path), `src/app/file_ops.rs`, `src/path_utils.rs`, `locales/*.yaml`.

**Acceptance criteria.**
1. Unit test: `resolve_local_image_path("image.png", Some(doc_dir), None)` and `"./image.png"` resolve a file **beside** the document (not only under `assets/`).
2. Manual: doc referencing a missing image → no per-frame warn spam in `--log-level debug`; replace `assets/sample.png` on disk → preview updates within a second.
3. Drop two same-named PNGs from different folders → two files, two distinct links.
4. Untitled tab + paste image → toast, nothing written.
5. CHANGELOG Fixed (#182) referencing both the 0.3.0 root cause and the closeout items.

**Complexity hint:** 4

---

### 6.3 Separate fonts for Rendered view and Raw editor ([#176](https://github.com/OlaProeis/Ferrite/issues/176))

**Current code.** One `Settings.font_family: EditorFont` (`src/config/settings.rs:2011`; `EditorFont = Inter | JetBrainsMono | Custom(String)` at `:1552`) feeds both editors from a single split point `src/app/central_panel.rs:856` → `EditorWidget` (`:1180`, `:1794`) and `MarkdownEditor` (`:2226`, `:2716`). Custom fonts load by installed family name via font-kit (`src/fonts.rs:926 load_system_font_by_name`) into one slot `FONT_CUSTOM = "Custom"` (`fonts.rs:1067`, insert `:2134-2136`), are prepended to `FontFamily::Proportional` (`:2172-2178`), and get a named family `FontFamily::Name("Custom")` (`:2240-2246`). Two near-duplicate ~250-line builders exist: `create_font_definitions_with_settings` (`:2080-2320`) and `create_font_definitions_with_cjk_spec` (`:1838-2069`). Family resolution: `get_styled_font_family` (`:2816-2839`) / `get_base_font_family` (`:2842-2856`). Rendered view has 37 call sites of `get_styled_font_family(..., editor_font)`; code blocks use `FontFamily::Monospace` independently. HarfRust shaping (`ttf_bytes_for_font_id_shaping`, `:1103-1123`) is raw-editor only.

**Required behaviour.**
- Setting `rendered_font_family: Option<EditorFont>` (`None` = "Same as editor"; serde default). Raw editor keeps `font_family`.
- Settings → Appearance: new row "Rendered view font" with a "Same as editor" toggle + the same built-in/custom picker. Factor the ComboBox block in `src/ui/settings/appearance.rs:340-393` into `custom_font_picker(ui, &mut EditorFont, id_salt)` and reuse it. Register in `registry.rs` after `appearance.font_family` (`featured: false`, keywords: `preview font, rendered font, proportional`).
- **Multi-custom-font registry** in `fonts.rs` (also unblocks #185): replace the single `FONT_CUSTOM` slot with keys `"custom:<family name>"`; each requested custom font gets `FontFamily::Name("custom:<name>")` = `[font] + proportional_fallbacks` built **after** `add_cjk_fallbacks` / `add_complex_script_fallbacks` so CJK/complex/emoji fallbacks are inherited. Replace `CUSTOM_FONT_BYTES: Mutex<Option<&'static [u8]>>` with `Mutex<HashMap<String, &'static [u8]>>`; `ttf_bytes_for_font_id_shaping` looks up `Name(n)` in the map before falling back to Inter; leak bytes once per distinct family (skip when the key exists).
- Replace the `custom_font: Option<&str>` parameter on the five rebuild functions (`reload_fonts :2513`, `load_cjk_for_text :2613`, `load_complex_script_fonts_for_text :2709`, `preload_explicit_cjk_font_with_custom :249`, `setup_fonts_with_settings :2429`) with a `FontSelection { editor: Option<String>, rendered: Option<String>, terminal: Option<String> }` derived from `Settings` by one helper. Only the **editor** font is prepended to `Proportional` (preserves today's UI look).
- `get_styled_font_family` / `get_base_font_family`: `Custom(name)` arm returns `FontFamily::Name(format!("custom:{name}"))`; the 37 rendered call sites need no change.
- Wiring: `central_panel.rs:856` computes `rendered_font_family = settings.rendered_font_family.clone().unwrap_or_else(|| font_family.clone())` and passes it at `:2226`/`:2716`; `MarkdownEditor::with_settings` (`src/markdown/editor.rs:1328`) same; change detection at `central_panel.rs:3139/3158` also compares the rendered font.
- **Dedupe the two `create_font_definitions_*` builders first** (one private builder + two thin wrappers) — otherwise the new family is lost after the first lazy CJK rebuild.

**Key files.** `src/config/settings.rs`, `src/fonts.rs`, `src/app/central_panel.rs`, `src/app/mod.rs:357-378, 733-760, 2841-2847`, `src/ui/terminal_panel.rs:574`, `src/markdown/editor.rs:1328`, `src/ui/settings/appearance.rs`, `src/ui/settings/registry.rs`, `locales/*.yaml`.

**Acceptance criteria.**
1. Unit tests: `FontSelection` from settings; font definitions contain `Name("custom:X")` and `Name("custom:Y")` when two different custom fonts are selected; fallback order test extended (custom → Inter → JetBrains → emoji → CJK).
2. Manual: Raw = JetBrains Mono, Rendered = Inter (or a custom system font) → Split view shows both; switch language to Japanese text → CJK glyphs render in both panes; Settings → Recently changed lists the new row.
3. Existing font tests pass; no change for users with `rendered_font_family = None`.
4. CHANGELOG Added (#176); new `docs/technical/ui/rendered-raw-fonts.md` + `docs/index.md`.

**Complexity hint:** 5 · **Depends on nothing; #185 depends on this task's `FontSelection` / custom-font registry.**

---

### 6.4 Nerd Font glyphs in the integrated terminal ([#185](https://github.com/OlaProeis/Ferrite/issues/185))

**Current code.** Terminal font is hard-coded `FontId::monospace(size)` at `src/terminal/widget.rs:117` (cell metrics from `'M'`), `:804` (row render), `:918` (bold branch). Only `terminal_font_size` exists (`settings.rs:2361`, `ui/settings/terminal.rs:23`, `registry.rs:373`). Monospace fallback chain: JetBrains Mono → emoji (`register_emoji_font_fallback`, `fonts.rs:862-868`) → CJK → complex scripts. PTY output passes through `Terminal::poll()` (`src/terminal/mod.rs:341-386`, already builds `String::from_utf8_lossy` at `:350`). Cell width: `put_char` uses `ch.width().unwrap_or(1).max(1)` (`screen.rs:307`); PUA chars are width 1 (correct). `painter.text` at `widget.rs:923` is **not clipped** to the cell.

**Required behaviour (both options; (b) alone fixes the default setup with zero config).**
- **(a) Terminal font picker**: `Settings.terminal_font_family: Option<String>` (installed family name; `None` = JetBrains Mono). Built on §6.3's `FontSelection.terminal`: register under `"custom:<name>"` and create `FontFamily::Name(FONT_TERMINAL = "ferrite-terminal")` = `[custom?, JetBrainsMono] + monospace_fallbacks`. `TerminalWidget` gets a `font_family: FontFamily` field/builder; replace the three `FontId::monospace(..)` sites. `ui/settings/terminal.rs` gets `render_font_family` (reuse `custom_font_picker`), registered after `terminal.font_size`, hint text: "Pick a *Mono* Nerd Font (e.g. JetBrainsMono Nerd Font Mono, MesloLGS NF) for Powerlevel10k".
- **(b) Lazy Nerd-symbol fallback**: in `fonts.rs` add `NERD_FONT_LOADED: AtomicBool`, `fn is_pua_char(c)` covering U+E000–U+F8FF, U+F0000–U+FFFFD (Nerd Font v3 Material icons), and Powerline U+E0A0–U+E0D4; `pub fn needs_nerd_font(text) -> bool`; `fn load_nerd_font()` trying `["Symbols Nerd Font Mono", "Symbols Nerd Font", "SymbolsNerdFontMono-Regular"]` then scanning `list_system_fonts()` for names containing `"Nerd Font"` / ending `" NF"` / `" NFM"` (prefer `Mono`). `register_nerd_font_fallback(fonts)` pushes `FONT_NERD` onto `Monospace` and `ferrite-terminal` **after** emoji. Trigger: in `Terminal::poll()` after `:350`, `if !NERD_FONT_LOADED && fonts::needs_nerd_font(&s) { NERD_FONT_REQUESTED.store(true) }`; in `ui/terminal_panel.rs` after `poll_all()` (`:1180`) call `fonts::ensure_nerd_font_loaded(ctx, &selection)` (atomic flag → rebuild → `bump_font_generation` → `schedule_prewarm`, same as `load_complex_script_fonts_for_text`). Both builders must re-register the fallback when the flag is set so later CJK rebuilds keep it.
- **Cell clipping**: wrap `widget.rs:923 painter.text(...)` in `painter.with_clip_rect(cell_rect.expand2(vec2(0.0, 1.0)))` so double-wide icon glyphs from non-Mono Nerd Fonts cannot smear into the neighbour cell.
- If no Nerd Font is installed: no change, no toast (log debug once).

**Key files.** `src/fonts.rs`, `src/terminal/widget.rs`, `src/terminal/mod.rs`, `src/ui/terminal_panel.rs` (~556 widget construction, ~1180 trigger), `src/config/settings.rs`, `src/ui/settings/terminal.rs`, `src/ui/settings/registry.rs`, `locales/*.yaml`.

**Acceptance criteria.**
1. Unit tests: `is_pua_char` ranges; `needs_nerd_font("\u{e0b0}")` true / `"abc"` false; font definitions include `FONT_NERD` on `Monospace` after `FONT_EMOJI` when the flag is set.
2. Manual (Windows with a Nerd Font installed): PowerShell in the integrated terminal, `"`u{e0b0} `u{f015} `u{e702}"` → glyphs, not boxes, in one cell each; without any Nerd Font → boxes as before, no crash.
3. Picker: choose "JetBrainsMono Nerd Font Mono" → p10k prompt renders; choose a proportional font → grid still aligned via `'M'` metrics (documented caveat).
4. CHANGELOG Added (#185); new `docs/technical/terminal/terminal-nerd-fonts.md` + `docs/index.md`.

**Complexity hint:** 5 · **Depends on §6.3.**

---

### 6.5 Middle-mouse primary-selection paste on Linux ([#183](https://github.com/OlaProeis/Ferrite/issues/183))

**Current code.** egui 0.34 has no primary-selection API ([emilk/egui#5852](https://github.com/emilk/egui/issues/5852)); `egui-winit` only wraps the regular clipboard. `arboard = "3"` is already a dependency (`Cargo.toml:53`; `x11rb` present, `wl-clipboard-rs` **not** — the `wayland-data-control` feature is off) and exposes `SetExtLinux`/`GetExtLinux` with `LinuxClipboardKind::Primary`. Raw editor mouse handling: `FerriteEditor::ui` in `src/editor/ferrite/editor.rs` — `allocate_painter(.., Sense::click_and_drag())` `:1778`; press `:2317`; `drag_started()` `:2336`; `dragged()` `:2361`; `response.clicked()` `:2383` → `pos_to_cursor` (`ferrite/mouse.rs:129`); selection setters `set_cursor`/`set_selection`. Undo is one step per dirty frame (`widget.rs:509`/`:881`), so "move caret + insert" in one frame is a single undo step. Only existing Linux gating: `#[cfg(target_os = "linux")]` in `src/terminal/sound.rs`. Middle-click elsewhere (tab close `central_panel.rs:635`, terminal tabs, links `widgets.rs:4724`) uses different responses — no conflict.

**Required behaviour.**
- New module `src/platform/primary_selection.rs`: `pub fn set_primary(text: &str)` and `pub fn get_primary() -> Option<String>`; Linux body via `arboard::Clipboard` + `LinuxClipboardKind::Primary` (`SetExtLinux::clipboard(..)`, `GetExtLinux::clipboard(..)`), no-op stubs for other OSes. Keep one `Clipboard` in a `thread_local!`/`OnceLock<Mutex<_>>` — do not construct per call (X11 connection each time). Skip empty text.
- **Publish on selection change, debounced**: `FerriteEditor` fields `last_primary_selection: Option<(usize, usize)>` + `primary_publish_at: Option<Instant>`. After the mouse block (~`:2488`): if `has_any_selection()` and the primary (anchor, head) differs from the last published, arm a 150 ms timer; when elapsed **and no pointer button is down**, call `set_primary(&self.selected_text())`, store, `ctx.request_repaint_after(150ms)` while armed.
- **Middle-click paste**: after the `response.clicked()` block: `if response.middle_clicked() && !response.dragged() { if let Some(pos) = response.interact_pointer_pos() { if !is_in_fold_indicator_area(pos) { let c = self.pos_to_cursor(..); if let Some(text) = get_primary() { self.set_cursor(c); self.insert_text_at_all_cursors(&text); response.request_focus(); } } } }`. Respect Vim mode (`vim_state.should_insert_text()` pattern at `:2830`) and preview lock is N/A (raw editor only).
- Switch `:2336`/`:2361` to `drag_started_by(PointerButton::Primary)` / `dragged_by(Primary)` so a middle-drag no longer starts a selection.
- Setting `middle_click_paste: bool` (default `true`), shown only on Linux (`cfg!(target_os = "linux")` in the registry render), Settings → Editor.
- `Cargo.toml`: enable `arboard = { version = "3", features = ["wayland-data-control"] }` so native Wayland primary selection works where the compositor supports `wlr/ext-data-control`; errors are swallowed. Document that otherwise XWayland is used.

**Key files.** `src/platform/mod.rs`, new `src/platform/primary_selection.rs`, `src/editor/ferrite/editor.rs` (struct ~101-160; mouse block 2301-2488), `src/config/settings.rs`, `src/ui/settings/editor.rs`, `registry.rs`, `Cargo.toml`, `locales/*.yaml`.

**Acceptance criteria.**
1. Unit tests: debounce state machine (pure function on `(selection, now, pointer_down)` → publish?/next deadline); middle-click insertion at a computed cursor produces one undo step (`EditHistory` len +1).
2. Builds on Windows/macOS with the stubs (CI). Linux CI builds with the new arboard feature.
3. Manual (Linux X11 and/or Wayland): select text in Ferrite → middle-click in a terminal pastes it; select in a terminal → middle-click in Ferrite raw editor inserts at the click position; middle-click on the tab strip still closes the tab.
4. CHANGELOG Added (#183, Linux only); new `docs/technical/editor/primary-selection-paste.md` + `docs/index.md`.

**Complexity hint:** 3

---

### 6.6 Mermaid inline-fit polish (#165 review follow-ups)

Visible quality bugs in the shipped fit-to-pane work; one task. Not crashes, but users of the headline #165 fix will hit them immediately.

1. **Scene clip bleeds past the preview** — `src/markdown/widgets.rs:5751` uses `Scene::show`, which *replaces* the clip rect (`egui scene.rs:147,209`), so a fit-mode diagram half scrolled out paints over the status bar / bottom panel. Fix: capture `parent_clip = ui.clip_rect()` before `scene.show` and inside the closure `ui.shrink_clip_rect(transform_to_local * parent_clip)`; **or** replace `Scene` with a plain `set_transform_layer` sublayer (also fixes item 2).
2. **Wheel-scroll jitter** — `Scene::register_pan_and_zoom` consumes `smooth_scroll_delta` while the pointer is over the diagram and translates for that frame only (fresh `scene_rect` each frame) → the diagram jumps by one wheel tick per frame. Fix as in 1 (no `Scene` for static fit), or persist `scene_rect` in memory.
3. **Forward-edge lanes detach arrows** — `src/markdown/mermaid/flowchart/render/edges.rs:676-679`: lane offset grouped per layer pair (`compute_forward_edge_lanes` keys on `(from_main, to_main)`, `:51-58`) and unclamped → ±45 px on `A & B & C & D & E --> X & Y` while min node width is 80 px. Fix: group per `(from, to)` node pair and clamp to `±(min(from_w, to_w)/2 − 6)`.
4. **Autonumber badge hidden under the label** — `src/markdown/mermaid/sequence.rs:913,990`: badge at `y-18` overlaps the label painted `CENTER_BOTTOM` at `y-8`. Fix: badge at the arrow origin (`from_x + dir*12`, `y`) like Mermaid.js. Also `:1031`, `:1089` `*num += msg_step` → `saturating_add`.
5. **Sequence fit scale uses estimated text metrics** — `src/markdown/mermaid/mod.rs:280-282` vs `sequence.rs:660-666`: pass a `&dyn TextMeasurer` (an `EguiTextMeasurer::new(ui)` exists at both call sites) into `measure_mermaid_diagram`; same for the frontmatter title height (`mod.rs:324-329`).
6. **Measure runs twice per frame per block** (`widgets.rs:5294` and `:5727`) → sequence/gantt/gitgraph parsed 3× per visible frame. Compute once in `render_validated_mermaid` and pass `natural_size` down.
7. Low: `cache.rs:221` evicts an LRU entry even when `key` already exists (skip when `contains_key`); Fit/Native toggle + `MermaidBlockData` keyed by `("mermaid_block", start_line)` only (`editor.rs:5640`, `widgets.rs:5377`) — mix in `rendered_editor_id(tab.id)` and a source hash; `sugiyama.rs:846-847` can push a sibling negative (clamp `.max(margin)`), `:889-892` shift `.max(0.0)`; `sequence.rs:187` `starts_with("autonumber")` should match the first whitespace token; request a repaint when the render replaces an estimated cache entry (`mod.rs:262-277`); popup `layout_width: None` (`mermaid_popup.rs:447-453`) → pass the inline pane width; CHANGELOG compatibility note for `subgraph id Title` already present — add a parser warning when an edge endpoint equals a former first-token id.

**Acceptance.** Manual on `test_md/test_mermaid_issue_165.md`: scroll a fit diagram half out of view → nothing painted outside the preview; wheel over it → no jitter; FC-165a arrows start/end inside node boxes; autonumber badges readable at the arrow start; CJK sequence diagram fills the pane width. Unit tests for lane clamp, `saturating_add`, cache no-evict-on-update. CHANGELOG Fixed; update `docs/technical/mermaid/inline-fit-to-pane.md`, `sequence-autonumber-rendering.md`, `dense-flowchart-layout.md`.

**Complexity hint:** 6

---

### 6.7 Small review items bundle (app/editor)

One task; each item is ≤ 20 lines.

1. `src/app/title_bar.rs:266-291` — non-Windows fallback sends `Maximized(false)` + `OuterPosition` + `StartDrag` in one frame (the #153 race) and mixes outer/inner coords. Restore + reposition this frame, defer `StartDrag` one frame via a flag.
2. `src/fonts.rs:840-880` — emoji font: `load_system_font` skips `validate_font_bytes`; Apple Color Emoji `.ttc` forced to index 0; the ~180 MB file is re-read on every font rebuild; `catch_unwind` wraps a struct copy. Cache the `Arc<FontData>` in a `OnceLock`, explicitly allow-list collections with index selection, drop the no-op guard. Add a macOS manual-QA row (blank glyph check).
3. `src/markdown/editor.rs:938,950-955` — `arrow_nav_sel_key()` is process-global and never cleared; scope by `rendered_editor_id(tab.id)` and clear in `cleanup_rendered_editor_memory`.
4. `src/markdown/widgets.rs:1311-1321` — `map_displayed_to_raw` collapses entities inside code spans / link text that are painted undecoded → caret off by `consumed-1`. Skip the entity branch while inside code/link-text state.
5. `src/editor/outline.rs:806-808` — positional escape restore desyncs when formatting strip removes a placeholder; encode the index in the placeholder char (PUA base + idx).
6. `src/app/file_ops.rs:1593-1597` — homegrown date math yields month 13 / day 31 in asset filenames; use `chrono`/`time` (check deps) or `std` + a tested civil-date helper.
7. `src/app/line_ops.rs`, `file_ops.rs:2575`, `input_handling.rs:261` — line numbering uses `split('\n')` while ropey counts `\r`, `\x0B`, `\x0C`, U+0085, U+2028/9 as breaks → wrong target line in docs containing those. Prefer the editor's char index (`editor.cursor_char_index()`) over `(line, col)`.

**Acceptance.** Unit tests for 4, 5, 6, 7; `cargo test` green; CHANGELOG Fixed lines.

**Complexity hint:** 4

---

## 7. Tier B — Spellcheck MVP ([#184](https://github.com/OlaProeis/Ferrite/issues/184)) — **task 6.8**

**Decision (human, 2026-09-08):** ship in 0.3.1. Engine: `spellbook`. Bundle `en_US` in the binary (no lazy/system-dictionary discovery). **Off by default.** Raw editor only.

**Problem (reporter).** No spellchecking; wants misspelled words underlined with suggestions, like every other Markdown editor.

**Engine.** [`spellbook`](https://crates.io/crates/spellbook) 0.4.x (Helix editor team; pure Rust, `no_std`+alloc, sole dependency `hashbrown`, MPL-2.0, Hunspell `.aff`/`.dic` compatible). API: `Dictionary::new(&aff, &dic) -> Result<Dictionary, ParseDictionaryError>`, `dict.check(&str) -> bool`, `dict.suggest(&str, &mut Vec<String>)`, `dict.add(&str)`. 0.4.2 fixed several non-char-boundary slice panics in the suggester — **pin `spellbook = "=0.4.2"`** or newer 0.4.x after checking its changelog; the crate warns its API may still break. Size: code is negligible; `en_US.aff` (~3 KB) + `en_US.dic` (~700 KB, ~50k stems) add ~1–2 % to the 30 MB release binary. Acceptable per the human decision. Dictionary source: the SCOWL-derived `en_US` from LibreOffice / `wooorm/dictionaries` (MIT/BSD-style licences — copy the licence file next to the dictionary and list it in `docs/legal/` or the About → Licences view if one exists).

**Reusable infrastructure (verified).**
- Squiggles are generic and not LSP-specific: `DiagnosticEntry { start_line, start_col, end_line, end_col, severity, message, source }` — **0-indexed lines, 0-indexed char columns** (`src/lsp/state.rs:28-41`; stub mirror `src/lsp_stub.rs:23-31` must stay field-identical). Rendered by `render_diagnostic_squiggles` / `draw_squiggle` (`src/editor/ferrite/highlights.rs:458-633`, `Hint` = dotted style at `:500`), hover tooltip (`ferrite/editor.rs:2490-2535`, severity label `"hint"` at `:2510`), fed via `EditorWidget::diagnostics(Vec<DiagnosticEntry>)` (`src/editor/widget.rs:489`).
- Mermaid validation already uses this non-LSP path: `compute_mermaid_diagnostics` appended in `src/app/central_panel.rs:1071-1083` (and `:1537-1566` for Split). **Do not copy its pattern** (it re-parses the whole document every frame); spellcheck results must be cached and refreshed only when the document version changes.
- Raw context menu `show_raw_editor_context_menu` (`src/editor/widget.rs:972-1073`) has `&mut FerriteEditor`; word extent via `find_word_boundaries` (`src/editor/ferrite/selection.rs:16`); `unicode-segmentation` 1.11 and `unicode-script` 0.5 are already dependencies; config dir via `config::persistence::get_config_dir()`; a background-thread + channel pattern exists in `src/workers/` and `src/search`-style indexers.

**Required behaviour.**
1. **Settings** (`src/config/settings.rs`, serde defaults): `spellcheck_enabled: bool = false`; `spellcheck_language: String = "en_US"`; `spellcheck_dictionary_dir: Option<PathBuf> = None` (folder of extra `<lang>.aff`/`<lang>.dic` pairs; bundled `en_US` always available); `spellcheck_ignore_all_caps: bool = true`; `spellcheck_ignore_words_with_digits: bool = true`. Registry (`src/ui/settings/registry.rs`, section Editor, `featured: true` for the toggle, keywords `spell, spelling, dictionary, typo, squiggle`); UI in `src/ui/settings/editor.rs`: toggle, language ComboBox (bundled `en_US` + every `.dic` stem found in the dir), folder picker (rfd, same pattern as other path pickers), the two ignore toggles, and a "Personal dictionary: *N* words · Open file" row.
2. **Module `src/spellcheck/`** (cargo feature `spellcheck`, **in default features**):
   - `dictionary.rs` — `include_bytes!("../../assets/dictionaries/en_US.aff")` / `.dic`; `fn load(lang, dir) -> anyhow::Result<Dictionary>` (bundled first, then `dir/<lang>.aff|.dic` read as UTF-8 with a `chardetng`-free fallback: if `from_utf8` fails try `encoding_rs::WINDOWS_1252` — many Hunspell dictionaries declare `SET ISO8859-1`; check the `SET` line in the `.aff`). Loading runs on the worker thread only.
   - `tokenize.rs` — `pub fn words_to_check(line: &str, ctx: &mut LineContext) -> Vec<(usize /*char start*/, usize /*char end*/, &str)>`. `LineContext` tracks multi-line state across the scanned window: inside fenced code (```` ``` ````/`~~~`, any fence length), inside frontmatter (`---` at line 0), inside HTML block. Per line skip: indented code (≥4 spaces / tab when not in a list continuation — use the existing `stats.rs::is_list_item` helper to disambiguate), inline code spans, URLs/autolinks/emails (`https?://`, `www.`, `[a-z]+@`), link/image destinations `](…)` and reference definitions `[x]: …`, wikilink targets `[[target|alias]]` (check the alias only), HTML tags and entities, `{{…}}` embeds, math `$…$`/`$$…$$`, `@mentions`, `#tags`, words containing digits or `_`, ALL-CAPS ≥2 chars (setting), camelCase/PascalCase (an inner uppercase after lowercase), hex/uuid-looking tokens. Word boundaries via `unicode_segmentation::UnicodeSegmentation::split_word_bound_indices`; only check tokens whose first alphabetic char is `Script::Latin` (extend later); strip leading/trailing `'`/`’`/`-`; keep interior apostrophes (`don't`). Return **char** offsets (convert from byte indices once per line).
   - `worker.rs` — one `std::thread` owning the `Dictionary`, a `HashSet<String>` of personal + session-ignored words, and a `HashMap<String, Vec<String>>` suggestion cache (cap 2,000 entries). Channel protocol: `Request::Check { tab_id: usize, version: u64, first_line: usize, lines: Vec<String> }`, `Request::Suggest { word }`, `Request::AddWord(String)`, `Request::IgnoreWord(String)`, `Request::Reload { lang, dir }`, `Request::Shutdown`; `Response::Diagnostics { tab_id, version, first_line, diags: Vec<DiagnosticEntry> }`, `Response::Suggestions { word, list }`, `Response::Loaded { lang, word_count } | LoadFailed { lang, error }`. The worker drops a `Check` if a newer `version` for the same tab is queued (drain the channel before working). Personal words persist to `<config_dir>/spellcheck/user-words.txt` (one per line, UTF-8, appended atomically via temp+rename); loaded at startup and passed to `dict.add` for each.
   - `mod.rs` — `pub struct SpellcheckService` owned by `AppState` (`Option<_>`, created when the setting turns on, dropped with `Shutdown` when turned off): `request_check(tab_id, version, first_line, lines)`, `poll()` (drain responses each frame; store `HashMap<tab_id, SpellResult { version, first_line, diags }>`), `diagnostics_for(tab_id, version) -> Option<&[DiagnosticEntry]>` (returns `None` when the stored version is stale so no wrong-offset squiggles are ever drawn), `suggestions_for(word) -> Option<&[String]>`.
3. **Wiring** (`src/app/central_panel.rs`, next to the Mermaid append at `:1071-1083` and `:1537-1566`): when `settings.spellcheck_enabled` and the tab is a Markdown/text file: compute the visible line window from the editor's viewport (the raw editor exposes first/last visible lines through `FerriteEditorStorage`/scroll state — reuse whatever `render_diagnostic_squiggles` uses for `start_line..end_line`) expanded by ±60 lines; key `(tab.id, tab.content_version_or_edit_epoch, first_line, last_line)`; if it differs from the last requested key and ≥250 ms passed since the last content change (`debounce`), send `Request::Check` with those lines cloned (never the whole document). Append `diagnostics_for(tab.id, version)` to `tab_diagnostics`. Use `severity: Hint`, `message: format!("Unknown word: {word}")`, `source: Some("spell".into())`. Call `ctx.request_repaint()` when `poll()` produced new results.
4. **Context menu** (`src/editor/widget.rs:972-1073`): when right-clicking over a spell diagnostic (`diagnostic_at(cursor)` — the tooltip code at `ferrite/editor.rs:2490` already resolves this; factor it into a helper), prepend: up to 5 suggestions (each: `set_selection(word range)` → `insert_text_at_all_cursors(&suggestion)`, one undo step), separator, "Add *word* to dictionary" (`AddWord` + persist + drop matching diags immediately), "Ignore *word* this session" (`IgnoreWord`). Suggestions are fetched asynchronously: on menu open send `Suggest` if not cached; show "…" until `Suggestions` arrives (menu re-renders each frame). Keep all existing menu items after these.
5. **Rendered/Split preview**: no squiggles (no rendered squiggle path). State it in the doc and in the setting tooltip ("Raw editor only").
6. **Performance guard**: `draw_squiggle` does one `layout_no_wrap` per squiggle segment; cap spell diagnostics passed to the widget at 300 per frame (nearest to the viewport first) and note the cap in the doc. Tokenising ±60 lines is O(window) — never touch lines outside the window. Toggling the setting off must remove all squiggles within one frame.
7. **Status bar**: while `Loaded` has not arrived after enabling, show a small "Loading dictionary…" text in the status bar (existing status-bar text pattern); on `LoadFailed` show a toast with the error and turn the setting back off.

**Key files.** New `src/spellcheck/{mod.rs, dictionary.rs, tokenize.rs, worker.rs}`, new `assets/dictionaries/en_US.{aff,dic,LICENSE}`, `Cargo.toml` (dep + feature), `src/main.rs`/`src/lib.rs` (`mod spellcheck`), `src/state.rs` (`spellcheck: Option<SpellcheckService>`), `src/config/settings.rs`, `src/ui/settings/{editor.rs,registry.rs}`, `src/app/central_panel.rs` (two append sites), `src/app/mod.rs` (poll each frame; create/drop service on setting change; `Shutdown` on exit), `src/editor/widget.rs` (context menu), `src/editor/ferrite/editor.rs` (factor `diagnostic_at`), `src/ui/status_bar.rs` (or wherever status text lives), `locales/*.yaml` (`settings.editor.spellcheck.*`, `context_menu.spell.*`, `spellcheck.loading`, `spellcheck.load_failed`).

**Acceptance criteria.**
1. Unit tests — `tokenize.rs`: fenced code (both fence styles, nested lengths), indented code vs list continuation, inline code, URLs/emails, link destinations vs link text (`[speling](http://x)` → checks `speling` only), wikilinks, HTML tags/entities, frontmatter, math, camelCase/ALLCAPS/digits skipped, apostrophes kept, CJK/emoji lines yield zero tokens, **returned offsets are char offsets** (line with `日本語 speling` → start col 4).
2. Unit tests — `dictionary.rs`: bundled `en_US` loads; `check("hello")` true, `check("helo")` false; `suggest("helo")` contains `hello`; user-words file round-trip (add → reload → accepted).
3. Unit tests — `worker.rs`: two queued `Check`s for the same tab → only the newest version is answered; `diagnostics_for` returns `None` for a stale version.
4. `cargo test` green; `cargo build --no-default-features --features bundle-icon` still compiles (feature off path).
5. Manual: setting off → no squiggles, no worker thread (check Task Manager thread count or debug log). Turn on → "Loading dictionary…" then dotted squiggles under `speling` in Raw within ~0.5 s; typing removes/moves them without lag on `test_md/test_large_file.md` (or any ≥5 MB file); right-click → suggestions replace the word in one Ctrl+Z step; "Add to dictionary" clears it and survives restart; code blocks/URLs/`[[wikilinks]]`/`日本語` produce no squiggles; Split view preview shows none (documented); turn off → squiggles vanish immediately.
6. Binary size delta ≤ 1.5 MB (`cargo build --release`, compare before/after).
7. CHANGELOG Added (#184: "Spellcheck (off by default, English bundled, raw editor)") + `docs/technical/editor/spellcheck.md` + `docs/index.md`; README feature bullet.

**Complexity hint:** 7 · Independent of other tasks; touches `central_panel.rs` and `widget.rs` — serialize after 5.5/6.7 to avoid merge churn.

**Out of scope (v0.3.2):** rendered-view squiggles, non-Latin scripts, per-document language switching, grammar, auto-correct, multi-dictionary (e.g. en_US + nb_NO simultaneously).

---

## 8. Non-functional requirements

- **Crash safety:** every task touching string slicing must add a multi-byte unit test. Grep gate before finishing any Tier A task: `rg "\[start\.\.|\[.*\+ 1\.\.\]|rfind\(" <touched files>` and justify each hit.
- **Data safety:** rendered commits must never change container structure (`>`/list markers) or line endings.
- **Performance:** no new per-frame full-document clones; image/mermaid failure paths must be cached.
- **i18n:** every new user string via `t!()` with keys in **all** `locales/*.yaml` (cs, de, en, es, et, ja, nb_NO, pt, ru, zh_Hans; fallback is en).
- **Tests:** `cargo test` green per task; `cargo clippy` must not add warnings to touched files (baseline is 289 style warnings — do not fix unrelated ones).
- **Docs:** feature-based names under `docs/technical/`; `docs/index.md` updated in the update phase.

---

## 9. Orchestrator parsing notes

Parse **this file** (`031-closeout-prd.md`) into a **new** cyclopsctl tag/queue.

- **Exclude §4** (no tasks). Exclude Appendix A/B.
- One task per numbered §5.x / §6.x heading plus **§7 as task 6.8** (14 tasks). Keep the bundles (§5.7, §6.2, §6.6, §6.7) as single tasks with checklists — do not split them. §7 may be split into two subtasks (engine+tokenizer+worker with unit tests; UI wiring+context menu+settings) if the orchestrator needs smaller units, but both must land before the parent is done.
- **Priority order:** 5.1 → 5.2 → 5.4 → 5.3 → 5.5 → 5.6 → 5.7 → 6.1 → 6.2 → 6.3 → 6.4 → 6.5 → 6.6 → 6.7 → 6.8 (§7).
- **Dependencies:** `6.4 depends on 6.3` (shared `FontSelection` / custom-font registry). `6.5` needs a Linux build check but no other task. Everything else is independent.
- **File-conflict serialization:** 5.4 and 6.7(3,4) both touch `src/markdown/editor.rs` / `widgets.rs` — serialize. 5.3 and 6.7(7) both touch `src/app/input_handling.rs` — serialize. 6.3 and 6.4 both touch `src/fonts.rs` — already sequential. 6.8 touches `central_panel.rs`, `editor/widget.rs`, `ui/settings/*` — run last.
- Each task: acceptance criteria, key files, complexity hint, test strategy (`cargo test` + the listed unit tests), issue link where applicable.
- The human may tag after Tier A; Tier B tasks that miss the tag roll into v0.3.2 unchanged.

```text
5.1 #178 search crash        ─┐
5.2 mermaid parser panics     ├─ independent, run first
5.3 smart-paste slice panic   │
5.4 blockquote commit         │
5.5 settings widget ids       │
5.6 UTF-16 EOL                │
5.7 reload follow-ups        ─┘
6.1 #186 instances            independent
6.2 #182 images closeout      independent
6.3 #176 fonts ──► 6.4 #185 nerd fonts
6.5 #183 middle-click         independent (Linux)
6.6 mermaid fit polish        independent
6.7 small bundle              after 5.3 / 5.4 (shared files)
6.8 #184 spellcheck (§7)      last (shared central_panel / widget / settings files)
```

---

## 10. Documentation deliverables

| Doc | Purpose | § |
|-----|---------|---|
| `docs/technical/editor/search-in-files-offsets.md` | byte vs char contract for search results | 5.1 |
| `docs/technical/mermaid/mermaid-inline-validation.md` | panic-freedom contract + corpus test | 5.2 |
| `docs/technical/markdown/smart-paste.md` | byte/char note | 5.3 |
| `docs/technical/markdown/rendered-edit-container-prefix.md` | seed/commit prefix contract | 5.4 |
| `docs/technical/ui/settings-search-redesign.md` | registry model, id-scope rule, recents | 5.5 |
| `docs/technical/files/line-ending-preservation.md` | post-decode detection on all load paths | 5.6 |
| `docs/technical/files/reload-from-disk.md` | undo step, toast, large-file resync | 5.7 |
| `docs/technical/platform/single-instance.md` | setting, `--new-instance`, lock + handshake | 6.1 |
| `docs/technical/markdown/local-image-assets.md` | failure cache, untitled policy, table-cell limitation | 6.2 |
| `docs/technical/ui/rendered-raw-fonts.md` | dual font model, `FontSelection` | 6.3 |
| `docs/technical/terminal/terminal-nerd-fonts.md` | picker + lazy symbol fallback | 6.4 |
| `docs/technical/editor/primary-selection-paste.md` | Linux middle-click paste | 6.5 |
| `docs/technical/mermaid/inline-fit-to-pane.md` etc. | clip/jitter/lanes/badge fixes | 6.6 |
| `docs/technical/editor/spellcheck.md` | engine, tokenizer skip rules, worker protocol, personal dictionary, limits | 7 |
| `docs/index.md` | link every new doc | all |
| `CHANGELOG.md` | Fixed/Added per task | all |

---

## 11. Acceptance criteria (this PRD done when)

1. `cargo test` green; no new clippy warnings in touched files.
2. **#178:** CJK Search-in-Files finds matches and places the caret correctly; whole-word `é` find does not crash.
3. Mangled-Mermaid corpus test passes; half-typed flowcharts in Split never abort.
4. Blockquote/callout click-to-edit round-trips; smart-paste over a selection near CJK does not crash.
5. Settings sliders/pickers/text fields keep state; shortcut rebinding does not flood "Recently changed".
6. UTF-16 CRLF stays CRLF; Reload undo/toast/large-file resync correct.
7. **#186** setting + flag + atomic lock + handshake + canonical paths; **#182** closeout; **#176** dual fonts; **#185** terminal Nerd glyphs; **#183** Linux middle-click paste; **#184** spellcheck MVP (off by default, `en_US` bundled, raw editor); Mermaid fit polish — shipped or explicitly rolled to v0.3.2 in CHANGELOG Deferred.
8. `docs/technical/platform/v0.3.1-minimal-release-test.md` Tier 1 passes on Windows.

---

## Appendix A — Deferred / not planned

| Item | Target | Notes |
|------|--------|-------|
| Spellcheck: rendered-view squiggles, non-Latin scripts, grammar, multi-dictionary | v0.3.2 | MVP ships in §7 |
| Inline images inside table cells / headings | v0.3.2 | Alt-text + icon fallback ships in 6.2 |
| Wayland-native primary selection without data-control | — | arboard limitation; XWayland fallback documented |
| Positional `autonumber off`/re-enable semantics | v0.3.2 | Mermaid.js parity, low demand |
| Mixed-EOL per-line preservation | — | Dominant ending policy stands (#174) |
## Appendix B — Human release steps (generate NO tasks)

1. Run `docs/technical/platform/v0.3.1-minimal-release-test.md` (Tier 1 mandatory).
2. `CHANGELOG.md`: rename `## [Unreleased] — v0.3.1` → `## [0.3.1] - <date>`; `Cargo.toml` already `0.3.1`.
3. Commit the working tree (it is currently ~7,300 lines uncommitted); tag `v0.3.1`.
4. Close GitHub issues: #175, #177 (shipped, no further task); #178, #182, #186, #176, #185, #183, #184 when their tasks land; plus the `031-prd.md` Appendix B list.
5. Delete scratch files at repo root: `_assess_clippy_output.txt`, `_assess_test_output.txt`, `_review_diff1..5.txt` (untracked review artifacts).

---

*Authored 2026-09-08 from a five-area read-only review of the uncommitted `0.3.1-experimental` working tree plus GitHub issues #175–#186. Line numbers refer to that working tree and may drift by a few lines after earlier tasks land — search by symbol name.*
