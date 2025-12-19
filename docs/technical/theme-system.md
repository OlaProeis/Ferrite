# Theme System

This document describes the theming system for Ferrite, covering architecture, color definitions, theme management, and integration with egui.

## Overview

The theme system provides a comprehensive set of colors, fonts, and spacing values for consistent UI styling. It supports light and dark themes with proper contrast and accessibility, plus a System theme that follows the OS preference.

## Architecture

```
src/theme/
├── mod.rs       # ThemeColors struct and core types
├── colors.rs    # Color constants and utilities
├── light.rs     # Light theme egui::Visuals configuration
├── dark.rs      # Dark theme egui::Visuals configuration
└── manager.rs   # ThemeManager for runtime theme switching
```

### Key Components

1. **ThemeColors** - Main struct containing all color definitions
2. **BaseColors** - Background, borders, hover states
3. **TextColors** - Primary, secondary, muted, link, code text
4. **EditorThemeColors** - Headings, blockquotes, code blocks, tables
5. **SyntaxColors** - Code syntax highlighting colors
6. **UiColors** - Accent, success, warning, error, info

## ThemeManager

The `ThemeManager` centralizes all theme operations, handling switching, persistence, and application of themes to egui.

### Basic Usage

```rust
use crate::theme::ThemeManager;
use crate::config::Theme;

// Create manager with initial theme
let mut manager = ThemeManager::new(Theme::Dark);

// Apply theme to egui context
manager.apply(&ctx);

// Switch themes
manager.set_theme(Theme::Light);
manager.apply(&ctx);

// Toggle between light/dark
manager.toggle();
manager.apply(&ctx);

// Cycle through all themes: Light -> Dark -> System
manager.cycle();
manager.apply(&ctx);
```

### Efficient Theme Updates

```rust
// In your update loop, use apply_if_needed() for efficiency
// This only applies theme when changed or when System theme detects OS change
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.theme_manager.apply_if_needed(ctx);
    // ... rest of UI
}
```

### Theme Persistence

The ThemeManager works with Settings for persistence:

```rust
// When theme changes:
fn handle_set_theme(&mut self, theme: Theme, ctx: &egui::Context) {
    self.theme_manager.set_theme(theme);
    self.theme_manager.apply(ctx);
    
    // Save to settings
    self.state.settings.theme = theme;
    self.state.mark_settings_dirty();
}
```

### Keyboard Shortcuts

- **Ctrl+Shift+T**: Cycle through themes (Light → Dark → System → Light)

### Menu Integration

Themes are accessible via **View > Theme** menu with options:
- ☀ Light
- 🌙 Dark
- 💻 System

## Usage

### Getting Theme Colors

```rust
use crate::theme::ThemeColors;
use crate::config::Theme;

// Get colors for a specific theme
let colors = ThemeColors::light();
let colors = ThemeColors::dark();

// Get colors based on user setting
let colors = ThemeColors::from_theme(theme, &ctx.style().visuals);

// Use colors in UI
ui.label(RichText::new("Hello").color(colors.text.primary));
```

### Applying Theme to egui

```rust
use crate::theme::ThemeColors;

// Convert theme colors to egui Visuals
let colors = ThemeColors::dark();
let visuals = colors.to_visuals();
ctx.set_visuals(visuals);

// Or use the convenience method
let visuals = ThemeColors::visuals_for_theme(Theme::Dark, &ctx.style().visuals);
ctx.set_visuals(visuals);
```

### Using Color Constants

```rust
use crate::theme::{DARK_BACKGROUND, LIGHT_TEXT, ACCENT_BLUE_DARK};

// Direct color access
let bg = DARK_BACKGROUND;
let text = LIGHT_TEXT;
let accent = ACCENT_BLUE_DARK;
```

### Using Color Utilities

```rust
use crate::theme::{blend_colors, darken, lighten, with_alpha, contrast_ratio};

// Blend two colors (0.0 = first, 1.0 = second)
let mixed = blend_colors(Color32::WHITE, Color32::BLACK, 0.5);

// Darken/lighten by percentage
let darker = darken(color, 0.2);  // 20% darker
let lighter = lighten(color, 0.2);  // 20% lighter

// Set alpha (0-255)
let semi_transparent = with_alpha(color, 128);

// Check contrast ratio for accessibility
let ratio = contrast_ratio(text_color, bg_color);
assert!(ratio > 4.5);  // WCAG AA minimum
```

## Color Categories

### Base Colors

| Color | Light | Dark | Purpose |
|-------|-------|------|---------|
| background | #FFFFFF | #1E1E1E | Primary background |
| background_secondary | #FAFAFA | #252525 | Panels, cards |
| background_tertiary | #F5F5F5 | #2D2D2D | Inputs, code blocks |
| border | #C8C8C8 | #3C3C3C | Primary borders |
| border_subtle | #E6E6E6 | #323232 | Dividers |
| hover | #F0F0F0 | #323232 | Hover state |
| selected | #E6F0FF | #283C50 | Selection |

### Text Colors

| Color | Light | Dark | Purpose |
|-------|-------|------|---------|
| primary | #1E1E1E | #DCDCDC | Main content |
| secondary | #505050 | #B4B4B4 | Descriptions |
| muted | #787878 | #8C8C8C | Hints, placeholders |
| disabled | #A0A0A0 | #646464 | Disabled text |
| link | #0064B4 | #64B4FF | Hyperlinks |
| code | #505050 | #C8C896 | Inline code |

### Editor Colors

| Color | Light | Dark | Purpose |
|-------|-------|------|---------|
| heading | #0064B4 | #64B4FF | H1-H6 headings |
| blockquote_border | #C8C8C8 | #505050 | Quote borders |
| blockquote_text | #646464 | #B4B4B4 | Quote text |
| code_block_bg | #E9ECEF | #23272E | Code backgrounds |
| horizontal_rule | #C8C8C8 | #505050 | HR elements |
| list_marker | #646464 | #969696 | Bullets, numbers |
| table_border | #C8CDD2 | #3C414B | Table borders |
| table_header_bg | #F0F2F5 | #2D323C | Table headers |

### Syntax Colors

| Token | Light | Dark | Description |
|-------|-------|------|-------------|
| keyword | #AF00AF | #C678DD | if, else, fn |
| string | #008000 | #98C379 | String literals |
| number | #008080 | #D19A66 | Numeric literals |
| comment | #808080 | #5C6370 | Comments |
| function | #0000AF | #61AFEF | Function names |
| type_name | #006496 | #E5C07B | Types/classes |
| variable | #323232 | #E06C75 | Variables |
| operator | #505050 | #ABB2BF | Operators |

### UI Feedback Colors

| Color | Light | Dark | Purpose |
|-------|-------|------|---------|
| accent | #0078D4 | #64B4FF | Primary actions |
| accent_hover | #0064B4 | #82C8FF | Hover state |
| success | #28A745 | #4BD264 | Success states |
| warning | #FFC107 | #FFD232 | Warnings |
| error | #DC3545 | #FF6464 | Errors |
| info | #17A2B8 | #50C8DC | Information |

## Theme Selection

The `Theme` enum in `config::settings` controls theme selection:

```rust
pub enum Theme {
    Light,   // Always use light theme
    Dark,    // Always use dark theme
    System,  // Follow system preference
}
```

When `System` is selected, the theme follows egui's `Visuals::dark_mode` flag.

## Accessibility

Both themes are designed for WCAG AA compliance:

- **Light theme**: Minimum 4.5:1 contrast ratio for normal text
- **Dark theme**: Minimum 4.5:1 contrast ratio for normal text
- **Link colors**: Distinguishable from regular text
- **Error/warning colors**: Sufficient contrast against backgrounds

Use `contrast_ratio()` to verify contrast:

```rust
let ratio = contrast_ratio(colors.text.primary, colors.base.background);
assert!(ratio >= 4.5, "Text contrast too low!");
```

## Extending the Theme

### Adding New Colors

1. Add field to appropriate struct in `mod.rs`
2. Define light/dark values in respective `::light()` and `::dark()` methods
3. Update `light.rs` and `dark.rs` to apply to egui Visuals if needed

### Creating Custom Themes

```rust
impl ThemeColors {
    pub fn custom() -> Self {
        Self {
            base: BaseColors {
                background: Color32::from_rgb(20, 20, 30),
                // ... custom values
            },
            // ... other categories
        }
    }
}
```

## Files

| File | Purpose |
|------|---------|
| `src/theme/mod.rs` | Core types: ThemeColors, BaseColors, TextColors, etc. |
| `src/theme/colors.rs` | Color constants and utility functions |
| `src/theme/light.rs` | Light theme → egui::Visuals conversion |
| `src/theme/dark.rs` | Dark theme → egui::Visuals conversion |
| `src/theme/manager.rs` | ThemeManager for runtime switching and persistence |

## Tests

Run theme tests:

```bash
cargo test theme::
```

Key test coverage:
- Light/dark theme color values
- Theme detection (`is_dark()`)
- egui Visuals conversion
- Color contrast validation
- Color blending utilities
- Shadow and rounding values
