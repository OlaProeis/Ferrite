# Explicit repaint after deferred UI updates (ISS-24)

GrokBot on Linux/Xvfb + llvmpipe reported **stale frames until the pointer moved**: Find match counter, Settings checkboxes, and Settings search filtering could look wrong until a mouse move triggered another frame. The same QA run did **not** reproduce on real GPU hardware; treat the issue as **primarily environmental**, with a code-side safeguard.

## Root cause

Ferrite disables egui’s automatic repaint-on-widget-change in `FerriteApp::new`:

```rust
options.repaint_on_widget_change = false;
```

That reduces idle CPU use, but any state that changes **after** widgets have already been laid out in the same frame needs an explicit `ctx.request_repaint()` (or `request_repaint_after`) to schedule the next frame.

Common cases:

| Flow | Why a second frame matters |
|------|----------------------------|
| **Find** | Debounced `find_matches` runs in `render_dialogs` after the Find panel drew the match counter from old `FindState`. |
| **Settings** | Registry `render_entry` toggles values; checkbox chrome may not refresh until the next paint. |
| **Settings search** | `search_query` changes filter chips and result list after the search `TextEdit` pass. |

Find already used `request_repaint_after(150ms)` when the query changed; that wakes the debounce frame but not necessarily a follow-up after `find_matches` mutates state.

## Fix (v0.3.1 closeout)

Explicit `request_repaint()` at:

- `src/app/dialogs.rs` — after debounced `find_matches`; after Find next/prev
- `src/app/central_panel.rs` — when `SettingsPanelOutput.changed`
- `src/ui/settings/mod.rs` — search field `changed()`, filter chip clicks, each `render_entry` change

When adding new UI that updates app state outside the widget that displayed the old value (especially with `repaint_on_widget_change` off), schedule a repaint the same way.

## Manual verification

On a real display (not Xvfb): toggle Find options, Settings checkboxes, and type in Settings search **without moving the mouse**; the UI should update immediately.

## Related

- Find debounce: `src/app/dialogs.rs` (`find_search_pending`, 150 ms)
- egui option: `src/app/mod.rs` (`FerriteApp::new`)
- QA table: `grokbot/New/qa-report-v0.3.1-linux.md` (ISS-24)
