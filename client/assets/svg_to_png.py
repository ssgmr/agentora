#!/usr/bin/env python3
"""SVG to PNG export script for Agentora game assets."""

import os
import cairosvg

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SPRITES_DIR = os.path.join(SCRIPT_DIR, "..", "assets", "sprites")
TEXTURES_DIR = os.path.join(SCRIPT_DIR, "..", "assets", "textures")

# 32x32 sprites (Agent icons)
SPRITES = {
    "agent_idle.svg": ("agent_idle.png", 32, 32),
    "agent_selected.svg": ("agent_selected.png", 32, 32),
    "agent.svg": ("agent.png", 32, 32),
}

# 16x16 textures (terrain, structures, legacies)
TEXTURES = {
    "terrain_plains.svg": ("terrain_plains.png", 16, 16),
    "terrain_forest.svg": ("terrain_forest.png", 16, 16),
    "terrain_mountain.svg": ("terrain_mountain.png", 16, 16),
    "terrain_water.svg": ("terrain_water.png", 16, 16),
    "terrain_desert.svg": ("terrain_desert.png", 16, 16),
    "structure_default.svg": ("structure_default.png", 16, 16),
    "legacy_default.svg": ("legacy_default.png", 16, 16),
}


def export_svg_to_png(svg_path, png_path, width, height):
    """Convert SVG to PNG with specified dimensions."""
    cairosvg.svg2png(
        url=svg_path,
        write_to=png_path,
        output_width=width,
        output_height=height,
    )
    print(f"  Exported: {os.path.basename(svg_path)} -> {os.path.basename(png_path)} ({width}x{height})")


def main():
    print("=== SVG to PNG Export ===\n")

    # Export sprites
    print("Sprites (32x32):")
    for svg_name, (png_name, w, h) in SPRITES.items():
        svg_path = os.path.join(SPRITES_DIR, svg_name)
        png_path = os.path.join(SPRITES_DIR, png_name)
        if os.path.exists(svg_path):
            export_svg_to_png(svg_path, png_path, w, h)
        else:
            print(f"  SKIP: {svg_name} not found")

    # Export textures
    print("\nTextures (16x16):")
    for svg_name, (png_name, w, h) in TEXTURES.items():
        svg_path = os.path.join(SPRITES_DIR, svg_name)
        png_path = os.path.join(TEXTURES_DIR, png_name)
        if os.path.exists(svg_path):
            export_svg_to_png(svg_path, png_path, w, h)
        else:
            print(f"  SKIP: {svg_name} not found")

    print("\n=== Export complete ===")


if __name__ == "__main__":
    main()
