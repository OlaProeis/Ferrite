#!/usr/bin/env python3
"""
Ferrite Icon Generator

This script generates application icons at various sizes from an SVG source.
It also creates platform-specific icon formats (.ico for Windows, .icns for macOS).

Requirements:
    pip install pillow cairosvg

Usage:
    python generate_icons.py

Or to generate from a specific SVG:
    python generate_icons.py --source icon_source.svg
"""

import os
import sys
import argparse
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("Error: Pillow not installed. Run: pip install pillow")
    sys.exit(1)

# Icon sizes to generate
SIZES = [16, 32, 48, 64, 128, 256, 512]

# macOS iconset sizes (includes @2x retina variants)
MACOS_SIZES = [16, 32, 64, 128, 256, 512, 1024]

# Ferrite brand colors
COLORS = {
    'primary': '#2D5A27',      # Forest green - main brand color
    'secondary': '#4A7C43',    # Lighter green - accents
    'accent': '#8BC34A',       # Bright green - highlights
    'background_light': '#FFFFFF',
    'background_dark': '#1E1E1E',
    'text_light': '#FFFFFF',
    'text_dark': '#2D5A27',
}


def create_placeholder_icon(size: int, output_path: Path):
    """
    Create a placeholder icon with the Ferrite 'F' logo.
    
    The design features:
    - Rounded rectangle background in forest green
    - Stylized 'F' letter representing Ferrite
    - Clean, modern aesthetic that works at all sizes
    """
    # Create image with transparency
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Calculate dimensions
    padding = size // 8
    corner_radius = size // 6
    
    # Draw rounded rectangle background
    bg_color = COLORS['primary']
    draw.rounded_rectangle(
        [padding, padding, size - padding, size - padding],
        radius=corner_radius,
        fill=bg_color
    )
    
    # Draw the 'F' letter
    f_color = COLORS['text_light']
    margin = size // 4
    line_width = max(2, size // 12)
    
    # Vertical line of F
    x_left = margin + line_width
    y_top = margin + line_width
    y_bottom = size - margin - line_width
    draw.line([(x_left, y_top), (x_left, y_bottom)], fill=f_color, width=line_width)
    
    # Top horizontal line of F
    x_right = size - margin - line_width
    draw.line([(x_left, y_top), (x_right, y_top)], fill=f_color, width=line_width)
    
    # Middle horizontal line of F (shorter)
    y_mid = (y_top + y_bottom) // 2
    x_mid_right = x_left + (x_right - x_left) * 2 // 3
    draw.line([(x_left, y_mid), (x_mid_right, y_mid)], fill=f_color, width=line_width)
    
    # Save the image
    img.save(output_path, 'PNG')
    print(f"Created: {output_path}")


def create_ico(input_pngs: list, output_path: Path):
    """Create a Windows .ico file from multiple PNG sizes."""
    images = []
    for png_path in input_pngs:
        if png_path.exists():
            img = Image.open(png_path)
            images.append(img)
    
    if images:
        # Save as ICO (Pillow handles multi-resolution automatically)
        images[0].save(
            output_path,
            format='ICO',
            sizes=[(img.width, img.height) for img in images]
        )
        print(f"Created: {output_path}")
    else:
        print("Error: No PNG files found to create ICO")


def main():
    parser = argparse.ArgumentParser(description='Generate Ferrite application icons')
    parser.add_argument('--source', help='Source SVG file (optional, generates placeholder if not provided)')
    parser.add_argument('--output', default='assets/icons', help='Output directory')
    args = parser.parse_args()
    
    # Determine script location and output paths
    script_dir = Path(__file__).parent
    output_dir = script_dir if args.output == 'assets/icons' else Path(args.output)
    
    # Create output directories
    (output_dir / 'windows').mkdir(parents=True, exist_ok=True)
    (output_dir / 'macos').mkdir(parents=True, exist_ok=True)
    (output_dir / 'linux').mkdir(parents=True, exist_ok=True)
    
    # Generate PNGs at all sizes
    png_paths = []
    for size in SIZES:
        output_path = output_dir / f'icon_{size}.png'
        create_placeholder_icon(size, output_path)
        png_paths.append(output_path)
    
    # Generate Windows .ico
    ico_path = output_dir / 'windows' / 'app.ico'
    create_ico(png_paths, ico_path)
    
    # Copy largest icon for Linux
    linux_sizes = [16, 32, 48, 64, 128, 256]
    for size in linux_sizes:
        src = output_dir / f'icon_{size}.png'
        dst = output_dir / 'linux' / f'ferrite_{size}.png'
        if src.exists():
            Image.open(src).save(dst)
            print(f"Created: {dst}")
    
    print("\n=== Icon Generation Complete ===")
    print(f"PNG icons: {output_dir}/icon_*.png")
    print(f"Windows ICO: {ico_path}")
    print(f"Linux icons: {output_dir}/linux/ferrite_*.png")
    print("\nFor macOS .icns, use iconutil:")
    print("  mkdir -p AppIcon.iconset")
    print("  cp icon_16.png AppIcon.iconset/icon_16x16.png")
    print("  cp icon_32.png AppIcon.iconset/icon_16x16@2x.png")
    print("  # ... (see docs/branding.md for full instructions)")
    print("  iconutil -c icns AppIcon.iconset")


if __name__ == '__main__':
    main()
