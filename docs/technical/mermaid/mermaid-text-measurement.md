# Mermaid Text Measurement System

## Overview

Implemented accurate text measurement for Mermaid diagrams in Ferrite, replacing hardcoded character-based width estimation with actual egui font metrics. This ensures node labels, edge labels, and other text elements render without clipping or overlap across all diagram types.

## Key Files

- `src/markdown/mermaid.rs` - All Mermaid rendering and the TextMeasurer implementation

## Implementation Details

### TextMeasurer Trait

A backend-agnostic trait for measuring text dimensions:

```rust
pub struct TextSize {
    pub width: f32,
    pub height: f32,
}

pub trait TextMeasurer {
    fn measure(&self, text: &str, font_size: f32) -> TextSize;
    fn row_height(&self, font_size: f32) -> f32;
    fn measure_wrapped(&self, text: &str, font_size: f32, max_width: f32) -> TextSize;
    fn truncate_with_ellipsis(&self, text: &str, font_size: f32, max_width: f32) -> String;
}
```

### EguiTextMeasurer

The primary implementation using egui's font system:

```rust
pub struct EguiTextMeasurer<'a> {
    ui: &'a Ui,
}

impl TextMeasurer for EguiTextMeasurer<'_> {
    fn measure(&self, text: &str, font_size: f32) -> TextSize {
        let font_id = FontId::proportional(font_size);
        let galley = self.ui.painter().layout_no_wrap(
            text.to_string(),
            font_id,
            Color32::PLACEHOLDER,
        );
        TextSize::new(galley.size().x, galley.size().y)
    }
    // measure_wrapped uses painter().layout_job(LayoutJob::simple(...)) similarly
}
```

### Pre-computation Pattern

To avoid borrow checker conflicts with egui (immutable borrow for measurement vs mutable for painting), all text measurements are pre-computed before allocating the painter:

```rust
pub fn render_flowchart(ui: &mut Ui, ...) {
    // Pre-compute edge label sizes (immutable borrow)
    let edge_labels: HashMap<usize, EdgeLabelInfo> = {
        let text_measurer = EguiTextMeasurer::new(ui);
        // ... measure all labels
    };

    // Now allocate painter (mutable borrow)
    let (response, painter) = ui.allocate_painter(...);

    // Use pre-computed sizes for drawing
    for (idx, edge) in edges.iter().enumerate() {
        if let Some(info) = edge_labels.get(&idx) {
            // Draw with pre-measured dimensions
        }
    }
}
```

### Safety Multiplier

A 1.15x multiplier is applied to all text width measurements to account for font rendering variations:

```rust
let name_width = name_size.width * 1.15 + padding;
```

## Diagram Types Updated

| Diagram | Elements Using Dynamic Sizing |
|---------|------------------------------|
| Flowchart | Node widths, edge labels with truncation |
| Sequence | Participant box widths |
| State | State widths, transition labels |
| Mindmap | Node widths (recursive pre-measurement) |
| Class | Class name widths, member widths |
| ER | Entity widths, attribute widths, relation labels |
| User Journey | Replaced unsupported emojis with filled circles |

## Dependencies Used

- `egui` - Font system via `Ui::fonts()` and `Galley`

## Usage

The text measurement is automatically used when rendering any Mermaid diagram. The pattern is:

1. Create `EguiTextMeasurer::new(ui)` at the start of render functions
2. Measure all text that will be drawn
3. Store measurements in HashMaps keyed by element index or ID
4. Drop the measurer (releases immutable borrow)
5. Allocate painter and draw using stored measurements

## Tests

Run the mermaid tests:

```bash
cargo test mermaid
```

Key test functions:
- `test_text_measurer_trait` - Tests `EstimatedTextMeasurer` implementation
- `test_truncate_with_ellipsis` - Tests ellipsis truncation logic
- `test_layout_produces_valid_positions` - Tests flowchart layout with text measurement
- `mermaid::measure::*` - ISS-59 parity: measurer vs painter galley for CJK/Latin (see `cjk-label-measure-paint-parity.md`)

## Previous Implementation

Before this change, text widths were estimated using:

```rust
// Old: Character count × font_size × factor
let node_width = node.label.len() as f32 * font_size * 0.6;
```

This was inaccurate because:
- Characters have varying widths (e.g., 'i' vs 'W')
- Font metrics vary between fonts
- No account for actual rendering

## Notes

- The `EstimatedTextMeasurer` struct exists for unit testing without an egui context
- `row_height()` method is currently unused but kept for future wrapping features
- User Journey emojis (😫😕😐🙂😊) were replaced with filled circles because the default font doesn't support emoji glyphs
