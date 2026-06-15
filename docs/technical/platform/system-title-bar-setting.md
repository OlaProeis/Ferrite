# Optional System Title Bar (#115)

## Overview

Ferrite defaults to a **custom title bar** (borderless window, no native OS decorations). Users on **Linux and macOS** can opt into the **native system title bar** via Settings → Appearance → **Use system title bar**. The change requires a **restart**; it is not applied live.

On **Windows**, native decorations remain unsupported: the setting is shown disabled with a tooltip explaining that Ferrite uses custom window chrome due to rendering limitations (Intel GPU / borderless transparency workaround).

## Setting

| Field | Location | Default |
|-------|----------|---------|
| `use_system_title_bar` | `Settings` in `src/config/settings.rs` | `false` |

Runtime gate:

```rust
pub fn native_window_decorations_enabled(&self) -> bool {
    self.use_system_title_bar && !cfg!(target_os = "windows")
}
```

Persisted in `config.json` via the normal settings save path.

## Window chrome

`apply_window_chrome()` in `src/ui/window.rs` centralizes viewport decoration flags:

- **Native mode:** `with_decorations(true)` — no transparency workaround.
- **Custom chrome (default):** `with_decorations(false)` + `with_transparent(true)` for the winit/glow misalignment fix on some Windows GPUs ([egui #2770](https://github.com/emilk/egui/issues/2770)).

Applied at creation in:

- `src/main.rs` — primary window
- `src/app/windows.rs` — secondary document viewports

## Runtime behavior when native decorations are active

When `native_window_decorations_enabled()` is true:

| Component | Behavior |
|-----------|----------|
| `render_title_bar` | Skipped in `src/app/mod.rs` |
| `handle_window_resize` | Skipped in `mod.rs` and `windows.rs` |
| `consume_clicks_in_resize_zones` / `apply_cursor` | Skipped |

OS-provided title bar, resize edges, and window controls replace Ferrite custom chrome and borderless resize handling.

## Settings UI

`src/ui/settings.rs` — Appearance section:

- Checkbox: `settings.appearance.system_title_bar`
- Hint + restart note (Linux/macOS)
- Disabled checkbox + tooltip on Windows (`settings.appearance.system_title_bar_windows_tooltip`)

i18n keys in `locales/en.yaml` under `settings.appearance`.

## Related docs

- [Custom Title Bar](./custom-title-bar.md) — default custom chrome implementation
- [Window Resize](./window-resize.md) — borderless edge resize (inactive when native decorations enabled)
- [Windows Borderless Transparent Fix](./windows-borderless-transparent-fix.md) — why custom chrome uses `with_transparent(true)`

## Manual verification

**Linux / macOS**

1. Enable **Use system title bar**, restart Ferrite.
2. Confirm native title bar; resize, maximize, and close work.
3. Disable setting, restart — custom title bar returns.
4. Confirm setting persists across restarts.

**Windows**

1. Open Settings → Appearance.
2. Confirm checkbox is grayed out and tooltip describes the limitation.
