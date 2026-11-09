# Settings “All” overview scroll (ISS-14)

GrokBot on Xvfb reported that wheel-scroll in Settings with empty search and the **All** chip selected stopped at **Files › Auto-save delay**, with **Recent Files**, **Keyboard**, and **Terminal** unreachable. Real-display verification was required before treating this as a layout bug.

## What “All” actually shows

The **All** chip is not a full catalog of every registry entry. It is the default overview:

1. **Recently changed** — up to `MAX_RECENTLY_CHANGED` (8) ids, frozen for the visit (`overview_recent_ids`).
2. **Essentials** — registry rows with `featured: true` and `browsable()`, minus ids already shown in recents.

In registry order, the last Essentials row is **`files.auto_save`** (toggle, delay label, slider, presets). **Recent Files**, the keyboard shortcut table, and terminal settings are **not** featured; they appear only via category chips or search. Reaching “Auto-save delay” at the bottom of Essentials is therefore expected, not evidence that Keyboard/Terminal were clipped.

## Scroll layout fix

When the overview list is taller than the tab (many recents + Essentials), the content `ScrollArea` must use the **remaining height** below the header, search bar, and chips — not grow with content and get clipped by the central panel.

`SettingsPanel::show_inline` applies the same pattern as the outline panel and nav cheatsheet:

```rust
egui::ScrollArea::vertical()
    .id_salt("settings_content_scroll")
    .auto_shrink([false, false])
    .max_height(ui.available_height())
```

Nested `ScrollArea`s inside individual settings (e.g. syntax-theme combo dropdown) only affect wheel events while the pointer is over those widgets.

## Manual QA

On a real display: Settings tab, empty search, **All** selected — wheel from top through Essentials to the auto-save slider and preset buttons. Use **Files** / **Keyboard** / **Terminal** chips (or search) for non-featured rows.

## Key files

- `src/ui/settings/mod.rs` — content `ScrollArea`, `show_overview`
- `src/ui/settings/registry.rs` — `featured` flag and entry order

## Related

- Overview model and recents: [`settings-search-redesign.md`](settings-search-redesign.md)
- GrokBot row: `grokbot/New/qa-report-v0.3.1-linux.md` (ISS-14)
