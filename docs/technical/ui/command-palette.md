# Command Palette

Searchable command launcher accessible via **Alt+Space** (configurable). Addresses [#59](https://github.com/OlaProeis/Ferrite/issues/59) as an alternative to traditional text menus — all application actions are discoverable through fuzzy search.

## Architecture

| File | Purpose |
|------|---------|
| `src/ui/command_palette.rs` | UI widget: overlay, fuzzy search, keyboard/mouse navigation, rendering |
| `src/app/commands.rs` | Command registry: maps `ShortcutCommand` to palette metadata (icon, category) |
| `src/app/platform.rs` | Platform hooks: Windows `WH_KEYBOARD` hook to suppress Alt+Space system menu |
| `src/app/input_handling.rs` | Pre-render key consumption for non-conflicting shortcuts (Ctrl+Shift+P) |
| `src/app/central_panel.rs` | Palette rendering (during render) + `dispatch_palette_command()` |
| `src/app/mod.rs` | Deferred dispatch: stores pending command, executes after render |

## Key Design Decisions

### Deferred Command Dispatch

Commands selected from the palette are **not** dispatched during `render_central_panel()`. Instead, the selected `ShortcutCommand` is stored in `pending_palette_command` and dispatched after render in `update()`, alongside keyboard shortcuts. This prevents mid-render state mutations (e.g., `handle_toggle_view_mode()` changing `tab.view_mode`) that caused `STATUS_STACK_BUFFER_OVERRUN` crashes in release builds.

### Windows Alt+Space Suppression

Windows opens a system menu (Restore/Move/Size/Close) on Alt+Space for all windows, including borderless ones. This happens via `WM_SYSCOMMAND`/`SC_KEYMENU` before egui sees the key event. A `WH_KEYBOARD` thread hook installed via `SetWindowsHookExW` intercepts Alt+Space at the keyboard message level, blocks it (returns 1), and sets an `AtomicBool` flag. The app checks this flag each frame and toggles the palette.

This approach was chosen over WndProc subclassing (which failed due to winit window class restrictions) and `consume_key` (which only affects egui's input, not the OS).

### Mouse vs Keyboard Navigation

The palette tracks `last_mouse_pos` and only updates the selected row from mouse hover when the mouse has actually **moved**. This prevents a stationary mouse from overriding arrow key navigation — a common UX issue in custom list widgets.

### Palette-Only Commands

Commands like `OpenWorkspace` and `CloseWorkspace` have no default keyboard shortcut (they use a dummy `F12` binding). The palette hides the shortcut badge for these using `KeyBinding::has_modifiers()`.

## Data Flow

```
Alt+Space pressed
  ├─ [Windows] WH_KEYBOARD hook → blocks key, sets PALETTE_TOGGLED flag
  │   └─ update() checks flag → command_palette.toggle()
  └─ [macOS/Linux] consume_command_palette_key() → consume_key() from egui input
      └─ command_palette.toggle()

User selects command
  └─ central_panel: palette.show() → output.selected_command = Some(cmd)
      └─ stored in pending_palette_command (NOT dispatched mid-render)

After render (in update())
  └─ if pending_palette_command.take() → dispatch_palette_command(ctx, cmd)
      └─ routes to existing handler (handle_save_file, handle_toggle_view_mode, etc.)
```

## Fuzzy Search

Uses `fuzzy-matcher` crate (`SkimMatcherV2`). Search text is `"{category} {display_name}"` per command. Recent commands get a +100 score boost. Results capped at 15.

## Recent Commands

Stored in `VecDeque<ShortcutCommand>` (max 20). Persisted via `Settings::command_palette_recent` and restored on app startup. Front = most recent (duplicate entries are moved to front).

## Adding New Commands

1. Add variant to `ShortcutCommand` in `src/config/settings.rs`
2. Add to `all()`, `display_name()`, `category()`, `default_binding()`
3. Add icon in `src/app/commands.rs` → `icon_for_command()`
4. Add dispatch in `src/app/central_panel.rs` → `dispatch_palette_command()`
5. Add name in `src/ui/settings.rs` → `shortcut_command_name()`

Example: **Lock editing** (`TogglePreviewLock`) toggles per-tab preview lock via `handle_toggle_preview_lock()` in `navigation.rs` — see [Preview Lock](./preview-lock.md).
