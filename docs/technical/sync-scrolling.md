# Sync Scrolling System

## Overview

The sync scrolling system provides synchronized scroll position between Raw and Rendered views. When enabled, switching between view modes maintains the scroll position so users don't lose their place in the document.

**Supported File Types:**
- **Markdown files (.md)**: Full bidirectional sync (Raw ↔ Rendered)
- **Structured files (JSON, YAML, TOML)**: Full bidirectional sync (Raw ↔ TreeViewer)

## Architecture

### Module Structure

```
src/preview/
├── mod.rs           # Module exports
└── sync_scroll.rs   # Core sync scrolling logic
```

### Key Components

#### SyncScrollState

The main state machine that manages synchronized scrolling:

```rust
pub struct SyncScrollState {
    pub enabled: bool,
    mappings: Vec<BlockMapping>,
    scroll_origin: ScrollOrigin,
    last_scroll_time: Option<Instant>,
    config: SyncScrollConfig,
    // ... scroll tracking fields
}
```

#### ScrollOrigin

Enum to track who initiated the last scroll event, preventing feedback loops:

```rust
pub enum ScrollOrigin {
    Raw,       // Scroll from Raw editor
    Rendered,  // Scroll from Rendered view
    External,  // External navigation (outline panel)
    None,      // Idle state
}
```

#### BlockMapping

Maps source line ranges to rendered Y positions:

```rust
pub struct BlockMapping {
    pub source_lines: (usize, usize),  // 1-indexed
    pub rendered_range: (f32, f32),    // pixels
    pub block_type: BlockType,
}
```

## How It Works

### View Mode Switching

When the user switches between Raw and Rendered modes:

1. **Record current scroll position** - Store the scroll offset
2. **Calculate source line** - Convert scroll offset to source line number
3. **Translate to target mode** - Convert source line to target scroll position
4. **Apply scroll target** - Set pending scroll position for target view

### Line Mapping Algorithm

The system uses block-level granularity for mapping:

1. **With mappings**: Interpolate within blocks for precise positioning
2. **Between mappings**: Interpolate between adjacent blocks
3. **Fallback**: Use proportional calculation based on total lines/height

```rust
// Example: Convert source line to rendered offset
pub fn line_to_rendered_offset(&self, line: usize) -> f32 {
    // Find mapping containing this line
    if let Some(mapping) = self.mappings.iter().find(|m| m.contains_line(line)) {
        // Interpolate within the block
        let progress = calculate_progress(line, mapping);
        return interpolate(mapping.rendered_range, progress);
    }
    // Fall back to proportional calculation
    self.proportional_line_to_rendered(line)
}
```

### Debouncing

Scroll events are debounced to prevent:
- Feedback loops between views
- Excessive synchronization calls
- Janky scroll behavior

Default debounce: 16ms (~60fps)

## Configuration

### Settings

The `sync_scroll_enabled` setting in `Settings` controls this feature:

```rust
pub struct Settings {
    // ...
    pub sync_scroll_enabled: bool,  // Default: true
}
```

### User Interface

- **Ribbon**: Toggle button (🔗/⛓) in the View group
- **Settings Panel**: Checkbox in Editor section
- **Keyboard**: Currently no shortcut (can be added)

## Implementation Details

### App Integration

The sync scroll state is stored per-tab in `FerriteApp`:

```rust
pub struct FerriteApp {
    sync_scroll_states: HashMap<usize, SyncScrollState>,
    // ...
}
```

### View Mode Toggle Handler

```rust
fn handle_toggle_view_mode(&mut self) {
    if let Some(tab) = self.state.active_tab_mut() {
        let old_mode = tab.view_mode;
        let new_mode = tab.toggle_view_mode();
        
        if sync_enabled {
            // Get sync state for this tab
            let sync_state = self.sync_scroll_states.entry(tab_id)
                .or_insert_with(SyncScrollState::new);
            
            // Calculate target scroll position
            match (old_mode, new_mode) {
                (ViewMode::Raw, ViewMode::Rendered) => {
                    let source_line = sync_state.raw_offset_to_line(current_scroll, line_height);
                    self.pending_scroll_to_line = Some(source_line);
                }
                (ViewMode::Rendered, ViewMode::Raw) => {
                    let source_line = sync_state.rendered_offset_to_line(current_scroll);
                    self.pending_scroll_to_line = Some(source_line);
                }
            }
        }
    }
}
```

## Scroll Offset Tracking

Each editor component reports its current scroll offset back to the tab state:

| Component | Field | Updated |
|-----------|-------|---------|
| `EditorWidget` (Raw) | `tab.scroll_offset` | ✅ On every frame |
| `MarkdownEditor` (Rendered) | `tab.scroll_offset` via `scroll_offset` output | ✅ On every frame |
| `TreeViewer` (JSON/YAML/TOML) | `tab.scroll_offset` via `scroll_offset` output | ✅ On every frame |

This ensures `tab.scroll_offset` always reflects the current scroll position regardless of view mode.

## Future Enhancements

The infrastructure supports future features:

1. **Split View Sync**: Real-time bidirectional scrolling in side-by-side mode
2. **Visual Indicators**: Gutter overlay showing visible region in other view
3. **Smooth Animation**: Animated scroll transitions when switching modes
4. **Block-Level Mapping**: Build mappings from rendered AST for precision

## Testing

Unit tests are provided in `sync_scroll.rs`:

```rust
#[test]
fn test_raw_offset_to_line() { ... }

#[test]
fn test_line_to_rendered_with_mappings() { ... }

#[test]
fn test_rendered_offset_to_line() { ... }

#[test]
fn test_proportional_fallback() { ... }
```

## Related Documentation

- [View Mode Persistence](view-mode-persistence.md)
- [WYSIWYG Editor](wysiwyg-editor.md)
- [Editor Widget](editor-widget.md)
