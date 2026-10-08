# Ferrite v0.3.1 — GrokBot QA Close-out (PRD)

> **Status:** Formal PRD — folds into the **0.3.1 branch** alongside `prd-v0.3.1.md`.
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into the custom task orchestrator (cyclopsctl). **Not yet parsed — the orchestrator is started manually later.**
> **Source:** `grokbot/qa-report-v0.3.1-linux.md` (29 issues, Linux X11 / Xvfb + llvmpipe, 2026-10). Screenshots in `grokbot/qa-shots-*.zip`, logs in `grokbot/qa-logs.zip`.
> **Relationship:** Buckets **A** (ISS-1/3/4/8/10/11/17/20/27) and **B** (ISS-2/5) were fixed directly in commits `5325e9b` and `97347d3`. This PRD covers everything that needs design, cross-module work, or real-hardware re-verification: buckets **C** (keyboard/auto-close trio), **D** (config robustness), **E** (polish & hygiene), plus **ISS-6** (undo granularity), which turned out to be an app-wide design question rather than a split-view bug.

## 1. Overview

GrokBot ran the v0.3.1 total test plan on Linux and logged 29 findings. The quick, isolated ones are already in. What is left falls into four themes:

- **Settings data loss (ISS-9)** — one bad field in `config.json` throws away *every* setting and the file is overwritten with defaults on exit. This is the only remaining **major** item and the headline of this PRD.
- **Undo granularity (ISS-6)** — a preview code-block edit in Split view undoes one keystroke at a time. Investigation showed the whole app records one undo group per dirty frame (`Tab::EditHistory::record_operations` → one group per call; raw editor `record_external_edit_from_snapshot` → one group per frame). There is no typing coalescing anywhere, so this is a design decision, not a bug fix.
- **Keyboard/auto-close trio (ISS-7, 12, 18)** — backtick-fence typing explodes into reversed text, Esc/Ctrl+, do nothing in Settings, first keystrokes are dropped in Settings search. ISS-7 has a clear code smell (post-render auto-close re-fires); 12 and 18 are flagged by GrokBot as possibly synthetic-input artefacts and need a real-keyboard confirm **before** any code is written.
- **Session/recovery and Settings polish (ISS-13–16, 19, 21–26, 28, 29)** — a dozen minor items that are cheap individually but touch shared code (session.rs, settings registry, status bar, parser options, test harness).

**Theme:** "Nothing the user typed or configured is silently lost." Config, undo history, recovery files and keystrokes all have to be trustworthy before 0.3.1 ships.

### Issues mapped

| Issue | Severity (QA) | Scope | Tier |
|-------|---------------|-------|------|
| ISS-9 corrupt config wipes all settings | **major** | §5.1 | A |
| ISS-6 undo granularity / typing coalescing | blocker→design | §5.2 | A |
| ISS-7 backtick auto-close runaway | blocker (no crash) | §5.3 | A |
| ISS-12 Esc / Ctrl+, in Settings | minor (verify first) | §5.3 | B |
| ISS-18 dropped first keystrokes in Settings | minor (verify first) | §5.3 | B |
| ISS-21 closed tab reappears in recovery | minor | §5.4 | A |
| ISS-22 "differs from disk" banner sticks after undo | minor | §5.4 | A |
| ISS-23 stale recovery file after clean exit | minor | §5.4 | A |
| ISS-28 `cargo test` writes to real config dir | minor (hygiene) | §5.4 | A |
| ISS-14 Settings "All" view scroll stops early | minor (verify) | §5.5 | B |
| ISS-15 Keyboard chip live count wrong | minor | §5.5 | A |
| ISS-16 "Recently changed" excludes rebinds; cap 6 vs 8 | minor (doc/code) | §5.5 | B |
| ISS-19 no validation for terminal cmd / prompt regex / sound path | minor | §5.5 | A |
| ISS-13 no terminal shell selector (triage) | minor (triage) | §5.5 | C |
| ISS-29 emoji invisible in Settings search | polish | §5.5 | B |
| ISS-25 stale status bar (line count, `[NORMAL]`) | polish | §5.6 | A |
| ISS-26 bare URLs / `<https://…>` not link-styled | minor | §5.6 | A |
| ISS-24 repaint lag until pointer moves (Xvfb) | minor (environmental?) | §5.6 | B |

---

## 2. Goals

- A `config.json` with one bad field loads with **only that field** defaulted; a file that is not JSON at all is **preserved** (renamed `.corrupt-<timestamp>`), the user sees a toast, and the next save never destroys the user's last good file.
- A single logical edit (typing a run of characters, a preview code-block edit, a paste) undoes in **one** step; cursor moves / pauses / mode switches start a new group. Behaviour is identical in Raw, Rendered and Split.
- Typing ```` ```mermaid ```` character by character in Split view yields exactly that text.
- Session restore never resurrects a tab the user closed; the "differs from disk" banner reflects the current buffer; a clean exit leaves `recovery/` empty.
- `cargo test` is hermetic — no writes outside the target/temp dirs.
- Settings search/filter counts, validation feedback and the status bar are correct and visible.

## 3. Non-goals

- No new undo *UI* (history panel, branching). Coalescing rules only.
- No settings schema migration framework — per-field fallback via `serde_json::Value` is enough.
- No rewrite of the auto-close feature; fix the re-fire bug and add tests.
- No fix attempts for ISS-12/18/24 **until** reproduced on a real keyboard/display; if they do not reproduce, close them with a note in the QA report.
- ISS-13 (terminal shell selector) is a checklist-vs-code triage, not an implementation task, unless the human decides the selector should exist.
- HTML export colour fidelity for bat palette themes (`ansi`/`base16*`) is noted in §7 as follow-up, not in scope.

---

## 4. Current state (verified in code, 2026-10-08)

### 4.1 Config loading (`src/config/persistence.rs`, `src/config/settings.rs`)
- `load_config()` → `load_config_internal()` → `Settings::from_json_sanitized(&contents)` which is a **strict** `serde_json::from_str::<Settings>` followed by `sanitize()`. Any type error (e.g. `"font_size": "abc"`) fails the whole deserialize; `unwrap_or_warn_default` logs a WARN and returns `Settings::default()`.
- `save_config()` writes `config.json.bak` then **renames it over** `config.json`. The `.bak` therefore never survives — it is a temp file, not a backup. On exit the defaults are written over the user's file (the QA data-loss path).
- `Settings` is `#[serde(default)]` at struct level, so *missing* fields already default individually; only *wrong-typed* fields and *syntax errors* are fatal.
- There is no user-facing notification on config load failure (`show_toast` exists in `AppState`).
- Portable/data dir: `FERRITE_DATA_DIR` env var (`DATA_DIR_ENV_VAR`) redirects the whole config dir — this is the natural hook for hermetic tests (ISS-28).

### 4.2 Undo (`src/editor/ferrite/history.rs`, `src/state.rs`, `src/app/central_panel.rs`)
- Raw editor: `record_external_edit_from_snapshot` records one group per dirty frame. Multiple keystrokes in one frame coalesce by accident; otherwise every keystroke is its own group. `break_group` is a no-op.
- Split/preview: a preview block edit flows through `Content modified in split rendered pane, recorded for undo` once per keystroke (QA log: 15 entries for one edit) — the rendered pane pushes the whole-buffer diff per frame.
- There is **no** notion of "typing run" (same cursor advancing by one, no newline, time gap below threshold) anywhere. Editors like VS Code / Sublime group consecutive insertions and break on cursor jumps, pauses (~300–1000 ms), newline, or command boundaries.

### 4.3 Auto-close (`src/app/input_handling.rs`)
- `handle_auto_close_pre_render` handles the "type the closer over an existing closer" / wrap-selection cases.
- `handle_auto_close_post_render(pre_render_content, _)` runs after the editor and checks `content_len_diff == 1`; if the char at `cursor-1` is an opener it inserts the closer and sets `tab.pending_cursor_restore`. **`pending_cursor_restore` is set in several places but never consumed** — the caret therefore ends up *after* the inserted closer, so the next typed backtick is again "one char inserted, previous char is `` ` ``" → closer inserted again → runaway (66 inserts for one fence, reversed text because the caret lands behind the closers).
- The quote/backtick guard only checks "previous char alphanumeric"; it does not check "next char is the same closer" nor the fence case (three identical openers).

### 4.4 Session & recovery (`src/config/session.rs`, `src/app/mod.rs`)
- Per-tab recovery blobs live in `recovery/<tab_id>.json`; `prune_recovery_dir(valid_tab_ids)` exists but is only called in some paths; closing a tab does not delete its blob or remove it from the crash snapshot (ISS-21).
- Crash snapshot (`save_crash_recovery_snapshot_file`) is written periodically; closed tabs remain until the next full snapshot.
- Conflict banner (`recovery.conflict.*`) is set once on restore and never re-evaluated against the live buffer (ISS-22).
- Clean exit: `clear_crash_recovery_snapshot_file()` runs; per-tab blobs are only cleared via `clear_all_recovery_data()` on explicit discard, so a restored-then-undone tab leaves `recovery/12.json` (ISS-23).
- Tests construct `Settings` / session paths via `get_config_dir()` with no env override → real `~/.config/ferrite` is created (ISS-28).

### 4.5 Settings panel (`src/ui/settings/`)
- Chip live count: `registry::all_entries().filter(section && browsable && matches(query)).count()` — the **Keyboard** section is one registry entry (the whole shortcut table), so it counts 1 regardless of matching rows (ISS-15).
- `MAX_RECENTLY_CHANGED = 8` in `registry.rs`; rebinds are intentionally excluded (ISS-16 is a checklist mismatch + decision).
- Terminal startup command / custom prompt patterns / sound file: plain `TextEdit`s with no validation; `Regex::new` failure surfaces nowhere (ISS-19).
- Settings search `TextEdit` uses the default egui font set; colour emoji are not in the fallback chain → `Failed to find replacement characters` WARN each launch (ISS-29).
- "All" overview is one long `ScrollArea`; GrokBot could not wheel past *Files › Auto-save delay* on Xvfb (ISS-14) — unconfirmed on real hardware.

### 4.6 Status bar & rendered links
- Status-bar line count is cached and refreshed on certain events; Ctrl+Shift+D (duplicate line) goes through a path that does not bump it; Vim mode indicator is not cleared when Vim is disabled while a special tab is active (ISS-25).
- `MarkdownOptions::autolink = true` is passed to comrak, so bare URLs **are** parsed as links; the rendered widget path must be dropping the link style for autolinks (`<https://…>` and bare) while keeping it for `[text](url)` (ISS-26). Check `src/markdown/widgets.rs` / `editor.rs` link-rendering for a `is_autolink`/`LinkType` branch.

---

## 5. Functional requirements

### 5.1 Tier A — Config robustness (ISS-9) — headline

**Behaviour.**
1. **Per-field tolerant load.** Parse `config.json` to `serde_json::Value` first. If it is valid JSON but a field fails to deserialize into its `Settings` type, drop *that field only* (log at WARN with the field path and the offending value), keep every other field, then run `sanitize()`. Implementation sketch: deserialize into `serde_json::Map`, attempt `Settings` deserialize; on `serde_json::Error`, use the error's path (or iterate fields against `Settings::default()` serialized as a map) to remove the bad key(s) and retry; bound the retry loop.
2. **Syntax-corrupt file is preserved.** If the content is not valid JSON at all: rename it to `config.json.corrupt-<YYYYMMDD-HHMMSS>` (keep at most 3 such files), load defaults with system locale, and set a one-shot `AppState` flag so the UI shows a toast: *"Settings file was unreadable and has been reset. The original was kept as …"* (new locale key, en + translated locales).
3. **Real backup on save.** `save_config()` writes to `config.json.tmp`, then copies the current `config.json` to `config.json.bak` (true last-good copy), then renames `.tmp` → `config.json`. `.bak` is never written from an in-memory default that came from a failed load (guard: if this session started from a failed load and the user has not changed any setting, do not overwrite `.bak`).
4. **Toast for partial recovery.** When ≥1 field was dropped, toast: *"N setting(s) were invalid and reset to defaults."*

**Tests.** `{ not json` → file renamed, defaults loaded, flag set. `"font_size": "abc"` → `font_size` default, `theme` etc. preserved. Round-trip save keeps a `.bak` equal to the previous content. Existing `test_sanitize_*` unaffected.

### 5.2 Tier A — Undo coalescing (ISS-6)

**Behaviour.**
- Introduce a **typing-run grouping rule** at the single place groups are created (`EditHistory` / raw-editor snapshot recorder) so Raw, Rendered and Split share it:
  - Consecutive single-character insertions where the new cursor == previous cursor + 1 on the same line, with gap < `UNDO_COALESCE_MS` (default 700 ms, settings-free constant), **merge into the open group**.
  - Any of: newline, deletion after insertion (or vice-versa), cursor jump, selection change, paste, formatting command, mode switch, focus loss → **break** the group.
  - `break_group()` becomes real and is called at those boundaries.
- Preview code-block edit (the ISS-6 repro): the whole edit from focus-in to "Done" is **one** group regardless of length. Simplest: the rendered pane records a single group on commit (Done / focus-out) instead of per-frame diffs; the per-frame `Content modified in split rendered pane` log becomes one line.
- Redo mirrors the same groups.
- Status bar "Undo (N remaining)" reflects groups.

**Tests.** Type `hello` as 5 frames → one group. Type `he`, move cursor, type `llo` → two groups. Preview block edit of 14 chars → one group. Pause > threshold splits. Existing history tests updated for the new grouping.

**Docs.** New `docs/technical/editor/undo-coalescing.md` describing the rules; `docs/index.md` updated.

### 5.3 Tier A/B — Keyboard & auto-close trio (ISS-7, 12, 18)

**ISS-7 (Tier A, implement).**
- Make `pending_cursor_restore` actually restore the caret (or remove the field and position the caret directly) so after auto-close the caret sits **between** opener and closer.
- Auto-close must **not** fire when the next char already is the closer; typing the closer then skips over it (standard "type-through").
- Backtick-specific: do not auto-close when the previous char is also a backtick (fence building), and never when the line already has an odd/even run that would form a fence. Same rule for `"`/`'` runs.
- Log line `Auto-close: Inserted` must appear **at most once** per opener in the ISS-7 script.
- Tests: character-by-character ```` ```mermaid\nflowchart TD\n``` ```` in Raw and Split; `(` → `()` with caret inside; typing `)` skips.

**ISS-12 / ISS-18 (Tier B, verify first).**
- Step 1 (human or real-display agent): reproduce on a physical keyboard. Record result in the QA report row.
- If ISS-12 reproduces: Esc with no focused text field closes the Settings tab; Ctrl+, opens/focuses Settings (check `KeyCode::Comma` mapping and that the Settings tab does not swallow the chord). Gear toggling is fine.
- If ISS-18 reproduces: inspect `focus_search` / `request_focus()` ordering — the first frame after `request_focus()` may discard `Event::Text` because the `TextEdit` was not yet focused; fix by consuming pending text events in the same frame or by focusing one frame earlier. Same for the chord-capture popup's first chord.

### 5.4 Tier A — Session & recovery hygiene (ISS-21, 22, 23, 28)

- **ISS-21:** closing a tab removes its recovery blob and its entry from the crash snapshot (call `prune_recovery_dir` with the live set after every close; snapshot rewritten on tab close).
- **ISS-22:** after Restore Session, re-evaluate the conflict banner whenever the buffer changes: hide it when `buffer == on-disk content` (hash compare, cheap), show again if it diverges. Unit test on the comparison helper.
- **ISS-23:** clean exit (`Alt+F4`, File › Exit, window close with no unsaved changes) deletes every per-tab blob for tabs that are **not** dirty. Dirty tabs keep their blob (that is the crash-safety feature). Test: restore → undo to clean → exit → `recovery/` empty.
- **ISS-28:** all tests that touch config/session paths set `FERRITE_DATA_DIR` to a `tempfile::TempDir` (test helper `with_temp_data_dir(|| …)` that also serialises via a mutex since env vars are process-global). CI gate: a test that asserts `~/.config/ferrite` (or `%APPDATA%\ferrite`) is not created by the suite is impractical; instead grep for direct `get_config_dir()` use in tests and route them through the helper.

### 5.5 Tier A/B/C — Settings polish (ISS-13, 14, 15, 16, 19, 29)

- **ISS-15 (A):** the Keyboard chip count = number of shortcut **rows** whose command name / binding matches the query (reuse `keyboard::shortcut_command_name` + `display_string()`); other chips unchanged.
- **ISS-19 (A):** inline validation with a red hint under the field, no blocking:
  - Custom Prompt Patterns: compile each line with `regex::Regex::new`; show the first error message.
  - Sound file path: warn if the path does not exist or has an unsupported extension.
  - Terminal startup command: non-empty whitespace-only / unbalanced quotes → hint. Values are still saved (power users), only flagged.
- **ISS-16 (B):** decision point for the human: (a) include rebinds in "Recently changed" (store `shortcut:<Command>` ids and link to the Keyboard section), or (b) keep excluding them and fix the checklist. Either way align the cap: checklist says 6, code says 8 — pick 8 and update `docs/technical/platform/v0.3.1-total-test.md` row 25.
- **ISS-14 (B):** verify on a real display. If it reproduces, the "All" `ScrollArea` is probably given a fixed `max_height` or an inner `ScrollArea` steals the wheel; fix so all sections are reachable.
- **ISS-29 (B):** add the colour-emoji font (already bundled for the editor?) to the fallback chain used by the Settings `TextEdit`, or at minimum make the WARN go away by registering the replacement chars. Low priority.
- **ISS-13 (C, triage):** confirm whether a shell selector was ever planned (`prd-v0.3.0.md` terminal section). If not, remove row 29's "terminal shell" from the checklist; if yes, open a separate task (out of this PRD's scope).

### 5.6 Tier A/B — Rendered-view & status polish (ISS-24, 25, 26)

- **ISS-25 (A):** status-bar line count recomputes on any buffer change (hook the same dirty signal the minimap/outline use); Vim indicator clears immediately when Vim mode is turned off, on every tab kind including special tabs.
- **ISS-26 (A):** autolinks (`LinkType::Autolink`, `LinkType::Email`, bare URLs from the comrak autolink extension) render with the same link colour/underline/hover/click as inline links. Test via `parse_markdown` → rendered inline spans carry `is_link`.
- **ISS-24 (B):** reproduce outside Xvfb/llvmpipe. If repaint lag exists on real hardware, audit for missing `ctx.request_repaint()` after Find results / checkbox toggles / Settings field edits. If it is Xvfb-only, record it and close.

---

## 6. Task breakdown for the orchestrator

Suggested parse into **7 parent tasks** (lowest id first; each independently testable):

1. **Config robustness** — §5.1 (ISS-9). Files: `src/config/persistence.rs`, `src/config/settings.rs`, `src/app/mod.rs` (toast), `locales/*.yaml`. Tests in `persistence.rs`.
2. **Undo coalescing** — §5.2 (ISS-6). Files: `src/editor/ferrite/history.rs`, `src/editor/ferrite/editor.rs`, `src/state.rs`, `src/app/central_panel.rs` (split pane recording), `docs/technical/editor/undo-coalescing.md`, `docs/index.md`.
3. **Auto-close correctness** — §5.3 ISS-7. Files: `src/app/input_handling.rs`, `src/editor/ferrite/editor.rs` (`handle_auto_close`), `src/editor/widget.rs` (consume `pending_cursor_restore`). Tests in `input_handling.rs`.
4. **Session & recovery hygiene + hermetic tests** — §5.4 (ISS-21/22/23/28). Files: `src/config/session.rs`, `src/app/mod.rs`, `src/app/file_ops.rs`, test helper in `src/config/mod.rs` or `tests/common`.
5. **Settings polish (implement)** — §5.5 ISS-15, ISS-19, ISS-16 cap alignment. Files: `src/ui/settings/mod.rs`, `keyboard.rs`, `terminal.rs`, `registry.rs`, `docs/technical/platform/v0.3.1-total-test.md`.
6. **Rendered & status polish** — §5.6 ISS-25, ISS-26. Files: `src/ui/status_bar*.rs` (or wherever the line count lives), `src/markdown/widgets.rs`, `src/markdown/editor.rs`.
7. **Real-hardware verification sweep** — ISS-12, 14, 18, 24, 29, 13 triage. Output: updated rows in `grokbot/qa-report-v0.3.1-linux.md` (Status column), and follow-up subtasks only for items that reproduce. Also re-run the ISS-2/ISS-5 Linux paths from commit `97347d3` (not compilable on the Windows dev host).

Each task: run `cargo test --bin ferrite` and `cargo clippy --all-targets` before finishing; commit per task. Do not touch `current-handover-prompt.md` / `ai-context.md` during implementation (cyclopsctl phase rules).

---

## 7. Follow-ups noted during bucket A/B (not in this PRD's scope)

- **HTML export + bat palette themes:** `src/export/html.rs` feeds raw syntect styles to syntect's HTML writer; `ansi`/`base16*` themes export as near-black text. Needs a style-mapping pass using `resolve_syntect_color` (added in `5325e9b`). Small; could ride with task 6 if time allows.
- **`pending_cursor_restore` is dead** (set, never read) — fixed as part of task 3 above; mentioned here because it also affects Search-in-Files caret placement (ISS-4 was worked around in `widget.rs` instead).
- **Untracked review scratch files** at repo root (`_assess_*.txt`, `_review_diff*.txt`, `assets/.jpg`, timestamped assets) are pre-existing and were left alone.

## 8. Acceptance (ship gate for 0.3.1)

- All Tier A items in §5 implemented with tests; `cargo test` hermetic; clippy clean on touched files.
- Tier B items either implemented or closed with a "does not reproduce on real hardware" note in the QA report.
- GrokBot re-run of the ISS-2 / ISS-5 / ISS-6 / ISS-7 / ISS-9 scripts on Linux passes.
