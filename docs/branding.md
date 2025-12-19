# Ferrite Branding Guidelines

This document outlines the branding assets, usage guidelines, and icon generation process for Ferrite.

## Brand Identity

**Ferrite** - A fast, lightweight text editor for Markdown, JSON, and more.

### Brand Colors

| Color | Hex | Usage |
|-------|-----|-------|
| Forest Green | `#2D5A27` | Primary brand color, icon background |
| Light Green | `#4A7C43` | Secondary accents |
| Bright Green | `#8BC34A` | Highlights, active states |
| White | `#FFFFFF` | Icon text, light theme backgrounds |
| Dark | `#1E1E1E` | Dark theme backgrounds |

### Icon Design

The Ferrite icon features:
- **Rounded rectangle** background in forest green
- **Stylized "F"** letter in white
- **Clean, modern aesthetic** that scales well from 16px to 512px

The design works on both light and dark backgrounds and maintains legibility at small sizes.

## Asset Locations

```
assets/
├── icons/
│   ├── icon_16.png          # 16x16 PNG
│   ├── icon_32.png          # 32x32 PNG
│   ├── icon_48.png          # 48x48 PNG
│   ├── icon_64.png          # 64x64 PNG
│   ├── icon_128.png         # 128x128 PNG
│   ├── icon_256.png         # 256x256 PNG (used by eframe)
│   ├── icon_512.png         # 512x512 PNG
│   ├── generate_icons.py    # Icon generation script
│   ├── windows/
│   │   ├── app.ico          # Windows multi-resolution icon
│   │   └── app.rc           # Windows resource file
│   ├── macos/
│   │   └── AppIcon.icns     # macOS icon set
│   └── linux/
│       ├── ferrite.desktop  # Linux desktop entry
│       └── ferrite_*.png    # Linux icon sizes
└── web/
    ├── favicon.ico          # Web favicon (16x16, 32x32)
    └── favicon.png          # Web favicon PNG (64x64)
```

## Generating Icons

### Prerequisites

- Python 3.8+ with Pillow: `pip install pillow`
- Or ImageMagick for manual conversion

### Using the Generation Script

```bash
cd assets/icons
python generate_icons.py
```

This generates:
- PNG icons at all standard sizes (16-512px)
- Windows `.ico` file
- Linux icon copies

### Manual Generation (ImageMagick)

If you have a source SVG or high-resolution PNG:

```bash
# Generate PNGs from SVG
for size in 16 32 48 64 128 256 512; do
    magick convert icon_source.svg -resize ${size}x${size} icon_${size}.png
done

# Create Windows ICO
magick convert icon_16.png icon_32.png icon_48.png icon_64.png icon_128.png icon_256.png app.ico
```

### macOS .icns Generation

macOS requires a specific iconset structure:

```bash
mkdir -p AppIcon.iconset

# Standard sizes
cp icon_16.png AppIcon.iconset/icon_16x16.png
cp icon_32.png AppIcon.iconset/icon_16x16@2x.png
cp icon_32.png AppIcon.iconset/icon_32x32.png
cp icon_64.png AppIcon.iconset/icon_32x32@2x.png
cp icon_128.png AppIcon.iconset/icon_128x128.png
cp icon_256.png AppIcon.iconset/icon_128x128@2x.png
cp icon_256.png AppIcon.iconset/icon_256x256.png
cp icon_512.png AppIcon.iconset/icon_256x256@2x.png
cp icon_512.png AppIcon.iconset/icon_512x512.png
# Note: 512@2x requires 1024x1024 source

# Convert to .icns (macOS only)
iconutil -c icns AppIcon.iconset -o macos/AppIcon.icns
```

## Platform Integration

### Windows

The icon is embedded in the executable via `build.rs`:

1. Place `app.ico` in `assets/icons/windows/`
2. The build script automatically embeds it during compilation
3. Icon appears in: File Explorer, taskbar, Alt+Tab

### macOS

For `.app` bundles:

1. Place `AppIcon.icns` in `assets/icons/macos/`
2. Reference in `Info.plist`: `<key>CFBundleIconFile</key><string>AppIcon</string>`
3. Icon appears in: Finder, Dock, Cmd+Tab

### Linux

Installation locations:

```bash
# Desktop entry
cp ferrite.desktop ~/.local/share/applications/

# Icons (per XDG spec)
for size in 16 32 48 64 128 256; do
    mkdir -p ~/.local/share/icons/hicolor/${size}x${size}/apps
    cp ferrite_${size}.png ~/.local/share/icons/hicolor/${size}x${size}/apps/ferrite.png
done

# Update icon cache
gtk-update-icon-cache ~/.local/share/icons/hicolor/
```

### eframe Window Icon

The application loads the icon at runtime via `ui/icons.rs`:

```rust
use crate::ui::get_app_icon;

let app_icon = get_app_icon();
let viewport = ViewportBuilder::default()
    .with_icon(app_icon.unwrap_or_default());
```

The icon loader tries:
1. Embedded icon (with `bundle-icon` feature)
2. `assets/icons/icon_256.png` (development fallback)
3. Graceful degradation to no icon

## Web Favicon

For web builds or documentation sites:

```html
<link rel="icon" type="image/x-icon" href="/favicon.ico">
<link rel="icon" type="image/png" sizes="64x64" href="/favicon.png">
```

## Usage Guidelines

### Do

- Use the icon on solid color backgrounds
- Maintain minimum padding of 10% around the icon
- Use provided sizes; don't upscale small icons

### Don't

- Place the icon on busy or patterned backgrounds
- Modify the icon colors or proportions
- Add effects like shadows or gradients
- Use the icon for non-Ferrite applications

## Version History

- **v1.0** (2024-12-19): Initial icon design with "F" logo
