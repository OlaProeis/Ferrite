# Ferrite - AI Context

Rust (edition 2021) + egui 0.28 markdown editor. Immediate-mode GUI, no retained widget state.

## Architecture

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `app.rs` | Update loop, event dispatch, title bar | `FerriteApp` |
| `state.rs` | All application state | `AppState`, `Tab`, `TabState` |
| `editor/widget.rs` | Text editing, line numbers | `EditorWidget` |
| `markdown/editor.rs` | WYSIWYG rendered editing | `MarkdownEditor` |
| `markdown/mermaid/` | Native diagram rendering | Per-type modules |
| `ui/ribbon.rs` | Main toolbar/menu | Toolbar buttons, dropdowns |
| `ui/settings.rs` | Settings modal | Settings panel |
| `config/settings.rs` | Persistent settings | `Settings`, `Language` |
| `theme/manager.rs` | Theme switching | `ThemeManager`, `ThemeColors` |

## Rust Patterns Used

```rust
// Error propagation - use anyhow, not panic
fn load_file(path: &Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

// Option handling - always check, never unwrap in library code
if let Some(tab) = self.tabs.get_mut(self.active_tab) {
    tab.modified = true;
}

// Borrowing - prefer references over clone
fn process(text: &str) -> Vec<&str> {  // not String, not clone
    text.lines().collect()
}

// Saturating math for indices (prevents underflow)
let idx = line_number.saturating_sub(1);
```

## egui Patterns

```rust
// Immediate mode: UI rebuilds every frame, state lives in AppState
fn update(&mut self, ctx: &egui::Context) {
    // Read state → draw UI → handle response → mutate state
    if ui.button("Save").clicked() {
        self.save_current_tab();  // mutation happens after UI
    }
}

// Repaint scheduling - don't spin CPU
ctx.request_repaint_after(Duration::from_millis(100));  // idle
ctx.request_repaint();  // only when needed (user input, animation)

// Response chaining
let response = ui.text_edit_singleline(&mut self.search_term);
if response.changed() { self.do_search(); }
if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) { ... }
```

## Critical Gotchas

| Issue | Wrong | Right |
|-------|-------|-------|
| **Byte vs char index** | `text[start..end]` with char positions | `text.char_indices()` or byte offsets |
| **Line indexing** | Mixing 0-indexed and 1-indexed | Explicit conversion: `line.saturating_sub(1)` |
| **Line height** | Hardcoded `20.0` | Get from galley: `galley.rows[0].height()` |
| **CPU spin** | Always `request_repaint()` | Use `request_repaint_after()` when idle |
| **Clone abuse** | `data.clone()` everywhere | Borrow with `&data` when possible |

## Code Conventions

- **Logging**: `log::info!`, `log::warn!`, `log::error!` (not println!)
- **i18n**: `t!("key.path")` for UI strings, keys in `locales/en.yaml`
- **State mutation**: Modify `TabState` for per-tab, `AppState` for global
- **File organization**: One concept per file, group in module folders
- **Error messages**: User-facing via `show_toast()`, technical via `log::error!`

## Where Things Live

| Want to... | Look in... |
|------------|------------|
| Add keyboard shortcut | `app.rs` → `handle_keyboard_shortcuts()` |
| Add UI panel | `ui/` → new file, wire in `app.rs` |
| Add setting | `config/settings.rs` → `Settings` struct |
| Add translation | `locales/en.yaml` + use `t!("key")` |
| Change theme colors | `theme/light.rs` or `theme/dark.rs` |
| Modify markdown rendering | `markdown/editor.rs` or `markdown/widgets.rs` |
| Add mermaid diagram type | `markdown/mermaid/` → new module |

## Known Limitations (egui TextEdit)

These cannot be fixed without custom editor widget (planned v0.3.0):
- Multi-cursor text operations
- Code folding (hiding text)
- Perfect IME positioning
- Scroll state access for sync
