#!/usr/bin/env python3
"""Generate the Keepsake bundle icons from the source art.

The source image is at `assets/source-icon.png` (an isometric
vault building with a slate roof, teal windows, and a brass
shield emblem).  This script crops it to a square, scales it
to the sizes Tauri's bundler expects, and writes a
multi-resolution .ico for Windows.

To re-generate from a new source:

  1. Drop the new image at `assets/source-icon.png`.
  2. Run `python3 scripts/make-icons.py`.

Run from the repo root:

  python3 scripts/make-icons.py
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

SIZE = 1024
ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets" / "source-icon.png"
ICON_DIR = ROOT / "crates" / "keepsake-app" / "src-tauri" / "icons"


def make_icon(size: int) -> Image.Image:
    """Crop the source to a centered square and resize."""
    if not SOURCE.exists():
        raise FileNotFoundError(
            f"source icon not found at {SOURCE}; "
            "drop the source art at assets/source-icon.png"
        )
    img = Image.open(SOURCE).convert("RGBA")
    w, h = img.size
    side = min(w, h)
    left = (w - side) // 2
    top = (h - side) // 2
    img = img.crop((left, top, left + side, top + side))
    return img.resize((size, size), Image.LANCZOS)


def make_ico(out: Path) -> None:
    base = make_icon(SIZE)
    base.save(
        out,
        format="ICO",
        sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)],
    )


def main() -> int:
    ICON_DIR.mkdir(parents=True, exist_ok=True)
    full = make_icon(SIZE)
    full.save(ICON_DIR / "icon.png")
    full.resize((512, 512), Image.LANCZOS).save(ICON_DIR / "icon-512.png")
    full.resize((256, 256), Image.LANCZOS).save(ICON_DIR / "icon-256.png")
    full.resize((128, 128), Image.LANCZOS).save(ICON_DIR / "icon-128.png")
    full.resize((32, 32), Image.LANCZOS).save(ICON_DIR / "32x32.png")
    full.resize((32, 32), Image.LANCZOS).save(ICON_DIR / "icon-32.png")
    full.resize((128, 128), Image.LANCZOS).save(ICON_DIR / "128x128.png")
    full.resize((256, 256), Image.LANCZOS).save(ICON_DIR / "128x128@2x.png")
    full.resize((384, 384), Image.LANCZOS).save(ICON_DIR / "128x128@3x.png")
    make_ico(ICON_DIR / "icon.ico")
    print(f"wrote icons to {ICON_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())