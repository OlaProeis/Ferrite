# Ferrite v0.3.1 — GrokBot QA Round 3 Close-out (PRD)

> **Status:** Formal PRD — folds into the **0.3.1 branch** after `prd-v0.3.1-qa-round2-closeout.md` (round 2, tasks 1–33 landed in `221b6ae`).
> **Consumers:** Human orchestrator + AI implementation sessions. Parse into cyclopsctl (`cyclopsctl launch --prd docs/ai-workflow/prds/prd-v0.3.1-qa-round3-closeout.md`) **only for §6 tasks 1–4**; the direct fixes in §5.1 landed in `8c9c2d5`.
> **Source:** `grokbot/november/qa-rerun2-report-v0.3.1-linux.md` — GrokBot's re-run of `grokbot/New/rerun-v0.3.1-round2.md` on 2026-11-09 against `221b6ae` (HEAD `9effd76`): 98 rows → 65 PASS / 23 FAIL / 7 PARTIAL / 2 BLOCKED / 1 DEFERRED; new issues **ISS-64…ISS-75**; §6 PARTIALs. Evidence unpacked under `grokbot/november/{shots,logs,rerun2}/` (zips git-ignored).
> **Relationship:** Every ISS-64…75 is a *successor* of an earlier issue or of a round-2 change. Nothing in GrokBot's Actual/Expected text was edited; a FAIL stays a FAIL until code or a decision in §8 closes it.

## 1. Overview

Round 2 removed the two Linux crashers (ISS-1 deadlock, ISS-2 GDK abort — zero hangs/aborts across 98 rows) and most of the fix list held. What came back falls into four themes:

- **Rendered-edit session state (ISS-64, 65, 68) — the serious cluster.** One round-2 change (`apply_rendered_session_live_frame`: write *every* dirty block to `Tab::content` each frame) met one latent bug (`render_heading` seeded cold headings as *dirty*). Together they rewrote headings lossily on view switch (`*` on a clean document; 6376 → 6374 chars), wrote a stale heading into a blank line after a code-block commit, and split undo. Independently, `BlockRef::widget_id(ui)` depended on `Ui` nesting depth, so the arrow-navigation hand-off computed one id and the TextEdit used another — the previous block kept focus and re-grabbed the session. **All three root-caused and fixed in this working tree with regression tests.**
- **Raw-editor and settings plumbing (ISS-67, 72, 74, 75)** — single-line-selection Tab replaced text; the settings debounce was starved by per-frame geometry dirtying; the popup guard ran before the context menu existed in the frame; the config toast expired before the screenshot. **All fixed directly.**
- **Linux-gated (ISS-66, 69).** X11 image paste via `xclip` image/png owner, and YouTube Error 153 from the `ferrite-video://` custom-scheme origin under WebKitGTK. Neither can be exercised on the Windows dev host; ISS-69 is a concrete orchestrator task, ISS-66 got diagnostics and is Linux-verify-only.
- **Product decisions (ISS-70, 71, 73 + §6 PARTIALs).** Copy from a formatted paragraph yields markdown (raw-edit mode is by design for 0.3.1), rendered list Tab was never implemented (dead imports since the WYSIWYG PRD), and the `linkStyle … interpolate basis` fixture was a collinear chain that no spline can curve.

**Theme:** "A fix that passes its unit test must also pass the frame loop." Each regression here had green tests at the layer it was written for and failed one layer up — the frame-order, id-scope, or clock the test did not exercise.

### Issues mapped

| Issue | Earlier ref | Why the earlier fix did not hold | Severity (QA → triage) | Call | Status |
|-------|-------------|----------------------------------|------------------------|------|--------|
| ISS-64 arrow nav never leaves block; `X` lands in heading | ISS-1 / ISS-31, #170 | Round-2 fix never worked end-to-end: `BlockRef::widget_id` used `ui.id()` of the *calling* Ui, so scope-level `surrender_focus` / focus compare in `apply_pending_arrow_navigation` targeted a different id than the nested TextEdit. Harness key-repeat exposed it | blocker → **blocker** | fix 0.3.1 | **fixed (`8c9c2d5`)** — `rendered_session.rs` scope key |
| ISS-65 code-block edit writes duplicate heading into blank line; undo split; dropped chars | ISS-6 / ISS-40 | Round-2 regression: all-dirty live write-through committed a *cold-seeded* `Heading{line:49}` after the code-block commit shifted lines. Dropped chars = TextEdit re-grab from ISS-64 ids | blocker → **blocker** | fix 0.3.1 | **fixed (`8c9c2d5`)** — non-dirty seed, active-only live write, heading guard |
| ISS-66 X11 Ctrl+V PNG inserts nothing, no toast | ISS-5 / ISS-46 | Cannot determine from logs: the only log line is egui-winit's own `arboard paste error` (text read on key press). Ferrite's release path logs nothing — either the release never reached us or the focus guard exited silently | blocker → **blocker, Linux-verify-only** | verify on Linux with new `debug!` lines; Windows Snipping Tool manual | **diagnostics added**; row re-run |
| ISS-67 multi-cursor Tab indents wrong text | #177 / ISS-51 | Wrong assumption in #177: only *multi-line* ranges were block indents; a single-line range (log `(6,0)→(6,14)`) went to `insert_text_at_all_cursors` and replaced the selection. Mid-line caret insert on line 33 is by design | blocker → **major** (data not lost; Ctrl+Z restored) | fix 0.3.1 | **fixed (`8c9c2d5`)** — `is_line_indent_range` |
| ISS-68 clean doc → Rendered shows `*`, 3 × "Content modified" | new (shares cause with ISS-65) | Latent since `041d8ec` (dirty heading seed), surfaced by round-2 write-through | major → **blocker** (content changed 6376→6374) | fix 0.3.1 | **fixed (`8c9c2d5`)** — same change set as ISS-65 |
| ISS-69 Linux YouTube Error 153; offline black rects; Gdk-CRITICAL | ISS-2 | ISS-2 fixed the abort only. `ferrite-video://localhost/embed` is a non-HTTP origin on webkit2gtk → no `Referer`, invalid `origin=` → YouTube 153. Windows maps custom protocols to `https://ferrite-video.localhost`, so it never showed there | major → **major, Linux-gated** | orchestrator task 1 | open |
| ISS-70 copy from formatted paragraph gives raw markdown | new | Not a regression: clicking a formatted paragraph enters raw-source edit; Ctrl+C copies the source TextEdit | major → **minor** | by design 0.3.1; "copy as plain text" 0.3.2 | decision §8 |
| ISS-71 Tab on rendered list item does nothing | IND-9 (ISS-1 row) | Never implemented: `check_structural_keys` / `indent_list_item` are imported but unwired ("Structural Enter handling stays disabled"). Round-2 expected text ("unchanged") assumed a feature that does not exist | major → **minor** | defer 0.3.2 (feature task) | decision §8 |
| ISS-72 corrupt / invalid config: no toast | ISS-9 | Toast wiring is correct and fires (log shows rename); 5 s lifetime vs screenshot ~1 min after launch → observation artefact | major → **minor (observability)** | fix 0.3.1 | **fixed (`8c9c2d5`)** — 30 s + `info!` log |
| ISS-73 indexed `linkStyle … interpolate basis` still straight | ISS-57 | Parser and Catmull-Rom renderer are correct; the fixture is a collinear `A→B→C→D` chain, which is straight in Mermaid too | minor → **by design (fixture)** | fixture replaced with fan-out/fan-in | **fixed (fixture)** |
| ISS-74 settings toggles not persisted until exit | SI-4 (task 29) | Starved debounce: `update_window_geometry_for` marked settings dirty *every frame*, `note_change` re-anchored each call; plus two clocks (`settings_mono_ms` vs eframe elapsed) | major → **major** | fix 0.3.1 | **fixed (`8c9c2d5`)** |
| ISS-75 spell tooltip over context menu | SPL-3 (task 31) | `Context::any_popup_open()` only sees popups shown earlier in the same pass; the tooltip is built before `response.context_menu` | polish → **polish** | fix 0.3.1 | **fixed (`8c9c2d5`)** |
| §6 UX-9 Ctrl+click file link reveals parent | ISS-42 | Linux without a file manager: `reveal` falls back to directory index | — | by design | closed |
| §6 T3-44 single-line `<div align>` left-aligned | ISS-33 | Round-2 fixed the *content* slice only; `align` not applied to single-line form | — | defer 0.3.2 | task 4 |
| §6 SES-3 Untitled-only secondary window won't close | ISS-63 | Seen once, not retested; code path `handle_window_close_request → request_close_window` | — | investigate | task 3 |
| §6 EXTRA-SET-7 ja/zh keyboard headers + 4 rows English | ISS-56 | Locale keys missing | — | defer 0.3.2 | task 4 |
| §6 MMD-10, HTML-4 `<kbd>` chip, T1-4b grey UTF-16, T1-13 status lag | ISS-60 / task 32 / — | Cosmetic | — | defer 0.3.2 | task 4 |
| §6 MMD-1 hand-pan at fit, T1-8 dialog dismissed on tab close, SES-6 word-granular undo | ISS-8 / — / ISS-36 | Behaviour as designed | — | by design | closed |
| §6 SI-1 minimized primary not raised, SI-3 recovery half, TTL-6a drag | ISS-34 / task 28 / ISS-54 | WM / harness | — | re-test only | rerun doc |
| §6 fixture typo `A# GitHub HTML Subset Test` | `570264b` | — | — | fixed | **fixed (fixture)** |
| §6 `cargo test` 2228 Linux vs 2239 Windows | — | Exactly 11 `#[cfg(windows)]` tests exist (7 in `path_utils::tests::windows_tests`, 2 more in `path_utils::tests`, 2 in `vcs::git::tests`); names listed in the round-3 re-run doc | — | confirmed | closed |

---

## 2. Goals

- Opening or switching a document to Rendered **never** marks it modified and **never** changes `Tab::content`; `cargo test` proves byte identity after a live frame.
- Typing in a rendered code block affects **only** that block; one Ctrl+Z reverts the whole session; no other line changes.
- Arrow Up/Down walks block-to-block in Rendered and Split, including into and out of blockquotes and across wrapped paragraphs, with a single focus owner per frame.
- Tab on any non-empty raw selection indents the touched lines; Shift+Tab outdents; selected text is never replaced.
- Any Settings toggle is on disk within ~500 ms while the app keeps repainting; `pkill -9` one second later finds the new value.
- The config-load toast is visible long enough for a human and logged for a harness.
- No tooltip paints over a context menu on the frame it opens.
- On Linux, a trusted YouTube embed either plays or degrades to a labelled placeholder — never Error 153 or a black rectangle (task 1).
- Each ISS-64…75 and §6 row has a Status in the November report and a row in the round-3 re-run doc or a by-design decision in §8.

## 3. Non-goals

- No "copy as plain text" from rendered formatted paragraphs in 0.3.1 (ISS-70 → 0.3.2 enhancement).
- No structural Tab/Shift+Tab for rendered list items in 0.3.1 (ISS-71 → 0.3.2 feature; it has never shipped).
- No Catmull-Rom / spline changes: ISS-73 is a fixture problem.
- No fix attempts for SI-1 (xfwm4 minimized raise) or TTL-6a (synthetic drag) until reproduced on a real display.
- No change to egui-winit's clipboard handling; ISS-66 is diagnosed with Ferrite-side logs first.

---

## 4. Current state (verified in code 2026-11-09, HEAD `9effd76` + working tree)

### 4.1 Rendered session (`src/markdown/rendered_session.rs`, `src/markdown/editor.rs`)

- `BlockEditState { text, dirty, formatted_editing, pending_activation, layout_wrap_width }`; `on_text_changed` sets `dirty = true`. Until this round `render_heading` and the formatted/plain-text seeding paths used `on_text_changed` to pre-populate cold blocks, so **every rendered heading was dirty from the first frame**.
- `apply_rendered_session_live_frame` (round 2, `221b6ae`) iterated all dirty non-TableCell blocks and called `write_session_block_to_source`. For a heading that uses `heading_plain_text(...)`, which strips backticks and inline HTML → lossy rewrite each frame (ISS-68's 6376 → 6374). After a code-block commit inserted a line, a dirty `Heading { line: 49 }` whose source moved to line 50 was written into the new blank line 49 (ISS-65's duplicate `# What to check`), as its own undo record (split undo).
- `BlockRef::widget_id(ui) = widget_id_in_scope(ui.id())`. egui child ids are `parent.with(child)`, so `ui.id()` inside `horizontal()` / blockquote indentation differs from the scope-level `ui.id()` used by `apply_pending_arrow_navigation` and `surrender_focus`. Focus never transferred; the old TextEdit re-activated the session on the next frame (ISS-64), and its re-grab dropped keystrokes meant for the new block (the ISS-65 "only first char kept").
- **Now:** `seed_block_text` (non-dirty), `set_rendered_widget_scope` / `rendered_widget_scope` stored in temp memory inside the `push_id(editor_id) → push_id(source_epoch)` chain, live write restricted to the **active** dirty block, `heading_level_from_source → Option` with a `warn!` skip when the line is no longer a heading. Doc: `docs/technical/markdown/rendered-session-scope-and-seed.md`.

### 4.2 Raw editor Tab (`src/editor/ferrite/editor.rs`, `indent.rs`)

- `handle_tab_key` classified selections with `is_multi_line_range`; single-line ranges fell through to `insert_text_at_all_cursors`, which **replaces** the selection with the indent string. GrokBot's log `cursor sync changed: (6, 14) -> (6, 4)` shows the selection was `(6,0)–(6,14)` — one line, not three (xdotool shift-drag selected only line 7). **Now:** `is_line_indent_range = selection.is_range()`; any range indents its lines. Collapsed caret behaviour unchanged (line 33 `中文行    首` is correct VS Code semantics).

### 4.3 Settings persistence (`src/config/settings.rs`, `src/app/windows.rs`, `src/app/mod.rs`)

- `SettingsSaveDebounce::note_change` re-stamped `changed_at_ms` every call; `update_window_geometry_for` called `mark_settings_dirty()` every frame → the 500 ms window never elapsed while repainting (always true under the harness). `mark_settings_dirty` used `settings_mono_ms()`, the poll used eframe's `start_time.elapsed()`. **Now:** anchor on first unsaved change, geometry dirty only on change, one clock. Test `debounce_flushes_despite_per_frame_note_change`.

### 4.4 Config-load toast (`src/state.rs`, `src/app/mod.rs`)

- `show_pending_config_load_toasts` fires on the first frame with a 5 s lifetime; the harness screenshots ~60 s after launch. Log confirms the rename ran. **Now:** `CONFIG_LOAD_TOAST_SECS = 30.0` + `info!("Config load toast: …")`.

### 4.5 Popup guard (`src/app/mod.rs`)

- `any_popup_open` wrapped `Context::any_popup_open()` which reads *this pass's* `open_popups`; the editor tooltip is decided before `response.context_menu`. **Now:** `||` memory-tracked `Popup::is_any_open` `||` previous-frame snapshot written by `remember_popup_state_for_next_frame` at the end of `update()`.

### 4.6 Video embeds on Linux (`src/markdown/video_render.rs`)

- Relay page is served from the wry custom protocol `VIDEO_EMBED_PROTOCOL = "ferrite-video"` (`with_custom_protocol` + `with_url("ferrite-video://localhost/embed?v=…")`, lines ≈ 681–720, 872–873). On Windows, wry maps custom protocols to `https://ferrite-video.localhost/...` so the iframe has an HTTPS referrer and `origin=` value. On webkit2gtk the page origin stays `ferrite-video://localhost`: no `Referer` is sent, the `origin` query parameter is not an HTTP origin → YouTube **Error 153** ("embedder identity missing"). Offline: the WebView never gets `onerror`, leaves the GTK surface black (no placeholder). The 20 `Gdk-CRITICAL _gdk_frame_clock_freeze/thaw` lines come from WebKit's GTK widget being realized/unrealized while scrolled off-screen.

### 4.7 X11 image paste (`src/app/input_handling.rs`)

- egui-winit 0.34.2 handles Ctrl+V itself: it reads clipboard **text** on key press; with an image-only owner arboard returns `Unknown error … incorrect type received from clipboard`, egui-winit logs it (the only line in GrokBot's log) and emits **no** press event — only the `Key { pressed: false }` release. `PasteSignals::from_events` treats that release as the paste signal; `try_consume_clipboard_image_paste` then (a) exits silently if `memory.focused()` is not the raw editor's `last_widget_id`, (b) exits silently if `get_text()` returns non-empty text, (c) reads `get_image()` → inserts or toasts. GrokBot saw neither insert nor toast, so the path exited at (a) or before — or the release never arrived. **Now:** `debug!` at (a), (b) and the `ContentNotAvailable` branch. Right-click **Paste image** is a separate path and worked in October.

### 4.8 Rendered list Tab (`src/markdown/editor.rs`, `src/markdown/ast_ops.rs`)

- `check_structural_keys` and `indent_list_item` are imported ("planned WYSIWYG keyboard interactions") and `render_list_item_with_structural_keys` carries the comment "Structural Enter handling stays disabled". No code path calls `indent_list_item` from a key event. IND-9's expected text came from the #177 indent doc's sentence "rendered list Tab unchanged", which described a feature that never existed.

### 4.9 Mermaid `linkStyle … interpolate basis` (`src/markdown/mermaid/flowchart/`)

- `parse_link_style` handles index lists (`0,1,2`) and `interpolate basis`; `render/edges.rs` draws Catmull-Rom through the routed points. `test_mermaid_v031.md` §3 was `A --> B --> C --> D` laid out on one axis: all control points collinear → the spline **is** a straight line, as in Mermaid.js. Fixture now fans out/in (`A→B, A→C, B→D, C→D`) with a note.

---

## 5. Functional requirements

### 5.1 Direct fixes — landed in `8c9c2d5`

| ISS | Files | Regression tests |
|-----|-------|------------------|
| 64 | `rendered_session.rs` (`set_rendered_widget_scope`, `rendered_widget_scope`, `BlockRef::widget_id`, `surrender_focus` lock order), `editor.rs` (scope set in push_id chain) | `widget_id_is_scope_stable_across_nested_uis` |
| 65 / 68 | `rendered_session.rs` (`seed_block_text`), `editor.rs` (`render_heading`, `ensure_formatted_block_initialized`, `render_session_plain_text_block`, `seed_session_block_text`, `apply_rendered_session_live_frame` active-only, `heading_level_from_source → Option`, warn skip) | `test_live_frame_does_not_write_cold_seeded_headings`, `test_stale_heading_commit_into_blank_line_is_skipped`, `test_heading_level_from_source` |
| 67 | `editor/ferrite/editor.rs` (`is_line_indent_range`, `handle_tab_key`, `block_indent_or_outdent`) | `test_single_line_selection_tab_indents_line_instead_of_replacing`, `test_partial_single_line_selection_tab_indents_line`, updated `test_mixed_multiline_and_single_line_range_tab_no_corruption` |
| 72 | `app/mod.rs` (`CONFIG_LOAD_TOAST_SECS`), `state.rs` (`info!`) | existing `state::tests::test_pending_config_load_*` |
| 74 | `config/settings.rs` (`note_change` first-change anchor), `app/windows.rs` (geometry change-only), `app/mod.rs` (single clock) | `debounce_flushes_despite_per_frame_note_change` |
| 75 | `app/mod.rs` (`any_popup_open`, `popup_open_prev_frame_key`, `remember_popup_state_for_next_frame`) | `tooltip_suppressed_before_context_menu_renders_in_frame` |
| 66 (diag) | `app/input_handling.rs` (`debug!` on every silent exit) | existing `paste_signals_distinguish_press_release_and_paste_event` |
| 73 / typo | `test_md/test_mermaid_v031.md` §3, `test_md/test_github_html.md` line 1 | — |

Verification run (Windows, 2026-11-09): `cargo test --bin ferrite` → **2246 passed, 0 failed, 2 ignored** (2239 baseline + 7 new tests). `cargo clippy --bin ferrite`: no new warnings in touched files (tree has 348 pre-existing).

### 5.2 Task 1 — Linux video-embed origin (ISS-69) — orchestrator

- In `video_render.rs`, on `cfg(target_os = "linux")` **do not** serve the relay page from `ferrite-video://`. Options in order of preference:
  1. Load `https://www.youtube-nocookie.com/embed/<id>?enablejsapi=0&rel=0&origin=https://ferrite-video.localhost` **directly** with `WebViewBuilder::with_url` and set the `Referer` through `with_headers` (wry supports request headers for the initial load on webkit2gtk). The relay page's only job (resize postMessage, click-to-play overlay) must then either be dropped on Linux or moved into a `with_initialization_script`.
  2. If headers are not honoured, bind a loopback `http://127.0.0.1:<port>/embed?v=` server (reuse the existing async worker infrastructure or `tiny_http` already in the tree if present — check `Cargo.toml` before adding a dependency) and point `with_url` at it; origin becomes a real HTTP origin.
- Offline / blocked network: the relay (or init script) must report `onerror` / a 5 s load timeout to Ferrite via IPC and the block renders the existing **placeholder** rect with the URL and a "Open in browser" button; no black GTK surface.
- Gdk-CRITICAL frame-clock asserts: hide (`set_visible(false)`) WebViews whose block rect is outside the viewport instead of resizing them to zero; check `video_render.rs` culling path.
- Tests: unit test that the Linux embed URL has an `https://` origin and a `Referer`; Linux GrokBot rows T2-36, VID-1, EXTRA-YT-2/4/6/7 in the round-3 re-run doc.
- Docs: `docs/technical/markdown/video-embed-rendering.md` platform section + `docs/index.md` bullet.

### 5.3 Task 2 — X11 image paste follow-through (ISS-66) — Linux-only verification, then fix if needed

- Run T1-14 / IMG-X1 / IMG-X2 on Linux with `--log-level debug` and the new `Image paste: …` lines. Outcomes:
  - `skipped, focus … is not the raw editor` → the editor's `last_widget_id` is stale after an xdotool click or focus was never given; fix the guard to accept focus on the editor's *scroll area* / use `ctx.memory(|m| m.has_focus(editor_id))` with the widget id the editor actually registers.
  - `release-only frame but clipboard holds text` → arboard returned non-empty text for an image-only owner; tighten to `matches!(get_text(), Ok(t) if !t.trim().is_empty())` only when the error is **not** the X11 type mismatch.
  - no `Image paste:` line at all → egui-winit swallowed the release too; add a `ctx.input(|i| i.key_released(Key::V) && i.modifiers.command)` check on **raw** `winit` events via eframe's `raw_input_hook`, or read the image on the *press* ourselves before egui-winit's handler by checking `i.modifiers.command && i.key_pressed(Key::V)` in `handle_pre_render_shortcuts`.
- Either way the failure toast (ISS-46) must appear when the clipboard claims an image but the read fails. Regression test for whichever branch is changed; Windows: Snipping Tool → Ctrl+V manual (Part A of the quick manual test).

### 5.4 Task 3 — Secondary window with only a dirty Untitled note does not close (SES-3 observation)

- Reproduce: Ctrl+Shift+N new window → Ctrl+N (Untitled) → type → × / Alt+F4. Trace `handle_window_close_request → request_close_window` in `src/app/windows.rs` for the branch where every tab in the window is `is_untitled() && is_modified()`: the save-prompt dialog is probably routed to the **primary** viewport (dialog state is global) while the close request is cancelled for the secondary, with no dialog visible on the window that asked. Fix: open the dialog on the requesting viewport or move the window's tabs to the dialog's viewport before cancelling; a secondary window with only Untitled content and the Quick-note setting ON closes without prompt (ISS-38 semantics) or shows the prompt in *that* window.
- Test: existing window-close tests in `windows.rs` + a new case for Untitled-only secondary window. Row SES-3 in the round-3 re-run doc.

### 5.5 Task 4 — 0.3.2 cosmetic/locale bundle (may run after the tag; not a 0.3.1 gate)

- T3-44: apply `align` to single-line `<div align>` (parser already keeps the attribute for the multi-line form).
- HTML-4: `<kbd>` chip inside headings (inline HTML subset in `heading_inline_html`).
- EXTRA-SET-7: add the missing ja / zh_Hans keys (Keyboard group headers, 4 command rows, accent/colour/view-mode strings, exit-dialog body); extend `locale_keys_exist_in_all_yaml`.
- MMD-10: `alt` frame right-edge clip; alt label offset in the `autonumber off` variant.
- T1-4b: UTF-16 editor text drawn faint grey (text colour from a `Tab` flag?); T1-13: status-bar line count stale during duplicate/move-line (request a repaint after the structural edit).
- ISS-71 feature: wire `check_structural_keys` → `indent_list_item` / outdent on Tab / Shift+Tab for the active rendered list item (own doc, `docs/technical/markdown/wysiwyg-interactions.md` update).
- ISS-70 enhancement: Ctrl+C with a selection inside a rendered formatted paragraph copies the **display** text; keep Ctrl+Shift+C for source.

---

## 6. Task breakdown for the orchestrator

Parse into **4 parent tasks** (lowest id first):

1. **Linux video-embed origin (ISS-69)** — §5.2. Files: `src/markdown/video_render.rs`, `docs/technical/markdown/video-embed-rendering.md`, `docs/index.md`. Tests: URL/origin unit test; Linux rows in the re-run doc. Gate for 0.3.1 Linux.
2. **X11 image paste follow-through (ISS-66)** — §5.3. Files: `src/app/input_handling.rs` (guard / release detection), possibly `src/editor/widget.rs` (`last_widget_id`). Linux-only verification first; code only on the branch the log identifies. Gate for 0.3.1 Linux.
3. **Untitled-only secondary window close (SES-3)** — §5.4. Files: `src/app/windows.rs`, `src/state.rs`. Gate for 0.3.1 (data-loss class if the user force-quits).
4. **0.3.2 cosmetic / locale / feature bundle** — §5.5 (T3-44, HTML-4, EXTRA-SET-7, MMD-10, T1-4b, T1-13, ISS-71, ISS-70). Not a 0.3.1 gate; may be parsed into a `v0.3.2` tag instead.

Each task: `cargo test --bin ferrite`, `cargo clippy --bin ferrite` (new warnings only), rustfmt on touched files, commit per task. Do not edit `current-handover-prompt.md` / `ai-context.md` during implementation.

---

## 7. Follow-ups noted during round 3 (not in scope)

- `apply_rendered_session_live_frame` still commits via `record_rendered_live_write_through` per frame for the active block; a 50–100 ms debounce would halve undo-record churn on fast typing without changing semantics.
- `rendered_widget_scope` falls back to `ui.id()` when no scope was set — fine for tests, but a `debug_assert!` in debug builds would catch a future caller outside `show_rendered_editor`.
- egui-winit's silent swallow of Ctrl+V on clipboard read errors is an upstream issue worth filing (text-paste failure hides image-paste intent).
- GrokBot harness: the Batch-A screenshots were lost (no `scrot`); the brief should require `scrot` before the first row.

## 8. Decisions log

| Row | Decision | By | Date | Note |
|-----|----------|----|------|------|
| ISS-70 copy from formatted paragraph | **by design (0.3.1)** — 0.3.2 enhancement | triage | 2026-11-09 | Click-to-edit switches a formatted paragraph to its source TextEdit; Ctrl+C there copies markdown, exactly as the Raw pane would. "Copy as plain text" is a new capability (task 4). CPY-3 row stays FAIL against its expected text; filed as enhancement, not relaxed. |
| ISS-71 rendered list Tab | **defer 0.3.2 (feature)** | triage | 2026-11-09 | Never implemented; the "unchanged" wording in `indent-selection.md` was wrong and is corrected. IND-9 expected text should be rewritten for round 3 as "Tab in a rendered list item does nothing (not implemented until 0.3.2)" **only if** the human accepts the deferral. |
| ISS-73 indexed `interpolate basis` | **by design (fixture)** | triage | 2026-11-09 | Collinear chain cannot curve; fixture replaced with a fan-out diagram and an explanatory note. MMD-3 re-runs on the new fixture. |
| ISS-72 severity | **downgraded to minor (observability)** | triage | 2026-11-09 | Toast fires; harness screenshot timing. Lifetime raised to 30 s and logged. |
| ISS-67 severity | **downgraded to major** | triage | 2026-11-09 | Tier-1 row, but Ctrl+Z restored byte-for-byte and the trigger is a single-line selection; fixed anyway. |
| ISS-68 severity | **upgraded to blocker** | triage | 2026-11-09 | Char count 6376→6374 is a silent content change on view switch; same change set as ISS-65. |
| §6 UX-9 reveal parent | **by design** | triage | 2026-11-09 | `reveal` without a file manager → directory index. Windows Explorer selects the file. |
| §6 MMD-1 hand-pan at fit | **by design** | triage | 2026-11-09 | Nothing to pan when the diagram fits. |
| §6 T1-8 dialog dismissed by tab close | **by design** | triage | 2026-11-09 | Closing the other tab invalidates the modal's target; re-trigger works. |
| §6 SES-6 ` W1` undo leaves space | **by design** | triage | 2026-11-09 | Word-granular coalescing (round 1 ISS-6 rule). Quick-test A3 wording updated. |
| §6 SI-1 minimized raise, TTL-6a drag | **re-test on real display** | triage | 2026-11-09 | xfwm4 / synthetic drag. |

## 9. Acceptance (ship gate for 0.3.1)

- [ ] §5.1 direct fixes committed on `0.3.1-experimental`; `cargo test --bin ferrite` green on Windows **and** Linux (Linux count = Windows − `cfg(windows)` tests, list attached).
- [ ] Windows manual pass: Part A of `docs/technical/platform/v0.3.1-quick-manual-test.md` (round-3 rows) all PASS.
- [ ] Tasks 1–3 implemented with tests (task 4 may slip to 0.3.2).
- [ ] GrokBot re-run on Linux using `grokbot/november/rerun-v0.3.1-round3.md`: no FAIL on listed rows; Tier 1 all PASS or BLOCKED-by-environment. FAILs become ISS-76+, not PRD edits.
- [ ] November report §5 Status column, `grokbot/README.md`, `docs/index.md` updated (done in this round for the triage; re-update after the re-run).
