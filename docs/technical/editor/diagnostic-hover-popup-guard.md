# Diagnostic hover tooltips vs open popups (SPL-3)

Spellcheck “Unknown word” and other raw-editor diagnostic hovers use `egui::Tooltip::always_open` in `FerriteEditor` (`src/editor/ferrite/editor.rs`). They must not paint over an open context menu or popup.

## Helpers (`src/app/mod.rs`)

- `any_popup_open(ctx)` — wraps **`Context::any_popup_open()`** (frame `open_popups` on layers, including context menus). Do **not** use deprecated `Memory::any_popup_open` / `Popup::is_any_open`; those miss `Popup::open_bool` menus shown in the same frame.
- `should_show_diagnostic_hover_tooltip(ctx)` — `!any_popup_open(ctx)`.

The editor checks this before building diagnostic hover tooltip parts (spell `Hint`, LSP severities, etc.).

## Tests

Harness: `app::diagnostic_hover_tooltip_tests::tooltip_suppressed_while_popup_open` — `Context::run_ui`, `Popup::open_bool`, assert suppression inside the same frame (popup state is cleared at `end_frame`).

Filter: `cargo test --bin ferrite tooltip`.

## Related

- Spellcheck pipeline: `docs/technical/editor/spellcheck.md`
- Squiggle paint (unchanged): `docs/technical/editor/diagnostic-squiggle-wrap.md`
