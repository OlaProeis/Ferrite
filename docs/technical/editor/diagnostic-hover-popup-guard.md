# Diagnostic hover tooltips vs open popups (SPL-3)

Spellcheck “Unknown word” and other raw-editor diagnostic hovers use `egui::Tooltip::always_open` in `FerriteEditor` (`src/editor/ferrite/editor.rs`). They must not paint over an open context menu or popup.

## Helpers (`src/app/mod.rs`)

- `any_popup_open(ctx)` — true when **any** of three sources says a popup is open:
  1. **`Context::any_popup_open()`** — popups shown *earlier in the current pass* (`open_popups` on layers).
  2. **`Popup::is_any_open(ctx)`** — memory-tracked popups (`open_memory` / `SetOpenCommand`), which stay set one frame after the last `show`.
  3. **Previous-frame snapshot** — `remember_popup_state_for_next_frame(ctx)` runs at the end of `FerriteApp::update` and stores `ctx.any_popup_open()` under `popup_open_prev_frame_key()` in temp memory.
- `should_show_diagnostic_hover_tooltip(ctx)` — `!any_popup_open(ctx)`.

The editor checks this before building diagnostic hover tooltip parts (spell `Hint`, LSP severities, etc.).

### Why three sources (QA ISS-75 / SPL-3 re-fail)

The round-2 fix relied on (1) alone. The editor's hover tooltip is built *before* `response.context_menu(...)` in the same frame, so on the first frame after a right-click `Context::any_popup_open()` was still `false` when the tooltip decided to show — the context menu had not been registered yet. The result was exactly what GrokBot saw: tooltip painted over the freshly-opened menu. Memory-tracked state (2) and the previous-frame snapshot (3) both cover the ordering gap: once a menu was open in frame N, frame N+1 suppresses the tooltip regardless of widget order.

## Tests

Harness (`app::diagnostic_hover_tooltip_tests`):

- `tooltip_suppressed_while_popup_open` — `Context::run_ui`, `Popup::open_bool`, assert suppression inside the same frame (popup state is cleared at `end_frame`).
- `tooltip_suppressed_before_context_menu_renders_in_frame` — memory-opened popup (`open_memory(Some(SetOpenCommand::Bool(true)))`) is detected on the next frame before any `show`; closing via `Popup::close_id` re-enables the tooltip.

Filter: `cargo test --bin ferrite tooltip`.

## Related

- Spellcheck pipeline: `docs/technical/editor/spellcheck.md`
- Squiggle paint (unchanged): `docs/technical/editor/diagnostic-squiggle-wrap.md`
