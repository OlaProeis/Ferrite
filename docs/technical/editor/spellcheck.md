# Spellcheck (#184)

Hunspell-compatible spellcheck for the **raw** editor only. Off by default. Feature `spellcheck` is in Cargo default features; `cargo build --no-default-features --features bundle-icon` omits the module and the Settings UI.

Rendered and Split **preview** panes never draw spell squiggles. Out of scope: non-Latin scripts, per-document language, grammar, auto-correct, multiple simultaneous dictionaries.

## Cargo / assets

| Item | Location |
|------|----------|
| Crate | `spellbook = "=0.4.2"` (optional, `dep:spellbook`) |
| Bundled dict | `assets/dictionaries/en_US.aff`, `en_US.dic` (SCOWL / wooorm `dictionaries/en`) |
| Licence | `assets/dictionaries/LICENSE` (MIT AND BSD, SCOWL-derived) |

`encoding_rs::WINDOWS_1252` is already a dependency; used when a `.aff` `SET` line declares ISO-8859-1 and UTF-8 decode fails.

Loading (`Dictionary::new`) runs **only on the worker thread**.

## Dictionary — `src/spellcheck/dictionary.rs`

`Dictionary::load(lang, dir)`:

1. Bundled first for `en_US` / `en-US` / `en` via `include_bytes!`
2. Else `dir/<lang>.aff` + `dir/<lang>.dic`

API: `check`, `suggest`, `add`, `word_count`. Personal words persist to `<config_dir>/spellcheck/user-words.txt` (UTF-8, one word per line, temp+rename). `load_user_words` / `save_user_words` / `user_words_path`.

## Tokenizer — `src/spellcheck/tokenize.rs`

`words_to_check(line, &mut LineContext) -> Vec<(char_start, char_end, &str)>`.

`LineContext` tracks fence (`` ``` `` / `~~~`, any length), YAML frontmatter (`---` at document line 0), HTML blocks, and list continuation (indented code vs list via `DocumentStats::is_list_item`).

Per-line skips: inline code, URLs/emails, `](…)` / `[ref]:` destinations, wikilink **targets** (`[[page|alias]]` checks alias only), HTML tags/entities, `{{…}}`, `$…$` / `$$…$$`, `@mentions`, `#tags` (not `# Heading`), digits/`_`, ALL-CAPS ≥2, camelCase/PascalCase, hex/uuid-like tokens. Word bounds via `split_word_bound_indices`. First alphabetic char must be `Script::Latin`. Strip leading/trailing `'` / `’` / `-`; keep interior apostrophes (`don't`).

Offsets are **char** columns (0-indexed), matching `DiagnosticEntry`.

## Worker / service

`src/spellcheck/worker.rs` — one `std::thread` (`ferrite-spellcheck`) owns the `Dictionary`, a `HashSet` of personal + session-ignored words, and a suggestion cache capped at 2,000.

`Request`: `Check { tab_id, version, first_line, lines }`, `Suggest`, `AddWord`, `IgnoreWord`, `Reload { lang, dir }`, `Shutdown`.

`Response`: `Diagnostics { …, diags: Vec<DiagnosticEntry> }`, `Suggestions`, `Loaded { lang, word_count }`, `LoadFailed { lang, error }`.

The worker drains the channel and keeps only the newest `Check` per tab. Diagnostics use `severity: Hint`, `message: "Unknown word: {word}"`, `source: Some("spell")`.

`SpellcheckService` (`src/spellcheck/mod.rs`): `request_check`, `maybe_request_check` (window key + 250 ms debounce), `poll`, `diagnostics_for(tab_id, version)` (**`None` if stored version ≠ requested** — never draw stale offsets), `suggestions_for`, `add_word` / `ignore_word` (drop matching cached diags immediately), `set_options`, `reload`, `shutdown` on `Drop`.

`AppState.spellcheck` is `Option<SpellcheckService>` behind `cfg(feature = "spellcheck")` and stays `None` until the setting is turned on.

`Request` also includes `SetOptions { ignore_all_caps, ignore_words_with_digits }`. Tokenizer defaults match those flags (`true`); `words_to_check_opts` honors them.

## Settings

| Field | Default |
|-------|---------|
| `spellcheck_enabled` | `false` |
| `spellcheck_language` | `"en_US"` |
| `spellcheck_dictionary_dir` | `None` |
| `spellcheck_ignore_all_caps` | `true` |
| `spellcheck_ignore_words_with_digits` | `true` |

Registry id `editor.spellcheck` (section Editor, `featured: true`, keywords `spell, spelling, dictionary, typo, squiggle`). UI in `src/ui/settings/editor.rs`: master toggle (tooltip: raw editor only), language ComboBox (`en_US` plus every `.dic` stem in the extra folder), rfd folder picker, the two ignore toggles, and “Personal dictionary: N words · Open file”.

## UI wiring

`FerriteApp::sync_spellcheck_service` (`src/app/mod.rs`): create the service when the setting turns on; `reload` / `set_options` when language, dir, or ignore flags change; `poll` each frame and `request_repaint` on new results; `LoadFailed` toasts `spellcheck.load_failed` and turns the setting off; drop the worker when the setting is off or on exit (`AppState::shutdown`).

Raw / Split-raw (`src/app/central_panel.rs` `append_spellcheck_diagnostics`, beside Mermaid appends): Markdown or unknown/text tabs only. Visible line range from `get_ferrite_editor_mut` → `ViewState::get_visible_line_range`, expanded ±60 (`VIEWPORT_PAD_LINES`). Key `(tab_id, content_version, first_line, last_line)`; send `Check` with **those lines only** when the key changed and ≥250 ms since `tab.last_edit_time`. Append `diagnostics_for` capped at **300** nearest the viewport midpoint (`cap_diagnostics_nearest_viewport`). Take `AppState.spellcheck` out of the state before the `&mut Tab` editor borrow; restore after `show`.

`FerriteEditor::diagnostic_at` / `diagnostics_at` (`src/editor/ferrite/editor.rs`) factor the hover tooltip lookup. Context menu (`src/editor/widget.rs` `prepend_spellcheck_menu_items`): up to 5 suggestions via `replace_word_range` (one `EditHistory` group); “Add word to dictionary” / “Ignore word this session”; `request_suggest_once` shows `…` until `Suggestions` arrives.

Status bar (`src/app/status_bar.rs`): small “Loading dictionary…” until `Loaded`. Locales: `settings.editor.spellcheck.*`, `context_menu.spell.*`, `spellcheck.loading`, `spellcheck.load_failed`.

Toggling off drops the service in the same frame — no spell `Hint`s are appended, so squiggles vanish immediately.

## Limits

- Tokenize only the visible window ±60 lines — never the whole document
- Cap spell diagnostics passed to the widget at **300** nearest the viewport
- Raw editor only; no Rendered/Split preview squiggles
- Toggling the setting off must drop the service (and squiggles) within one frame

## Tests

`tokenize.rs`: fences (both styles, nested lengths), indented code vs list, inline code, URLs/emails, `[speling](http://x)`, wikilink alias, HTML, frontmatter, math, camel/ALLCAPS/digits, `don't`, CJK/emoji, `日本語 speling` start col 4.

`dictionary.rs`: bundled load, `hello`/`helo`, `suggest("helo")` contains `hello`, user-words round-trip, ISO-8859-1 fallback.

`mod.rs` / `worker.rs`: two queued Checks → newest version only; `diagnostics_for` stale → `None`; `Shutdown` joins; window-key debounce; 300-nearest cap; locale keys in all `locales/*.yaml`.

Settings: `spellcheck_enabled` false / `en_US` defaults; old JSON without the fields deserializes. Context-menu replace: one `EditHistory` group (`spell_replacement_is_one_undo_group`).
