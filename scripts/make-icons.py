#!/usr/bin/env python3
"""Generate the Keepsake bundle icons.

A shield in deep teal with a bold white "K" centered on
top.  Outputs the PNG and ICO files Tauri's bundler
expects.

Run from the repo root:

  python3 scripts/make-icons.py
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageFilter

# Brand color: deep teal.
TEAL = (15, 109, 130, 255)
TEAL_DARK = (10, 78, 94, 255)
WHITE = (255, 255, 255, 255)
HIGHLIGHT = (40, 150, 175, 255)

SIZE = 1024
ICON_DIR = (
    Path(__file__).resolve().parent.parent
    / "crates"
    / "keepsake-app"
    / "src-tauri"
    / "icons"
)


def shield_mask(size: int) -> Image.Image:
    """Alpha mask shaped like a shield."""
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    margin = int(size * 0.02)
    body_bot = int(size * 0.72)
    radius = int(size * 0.22)
    d.rounded_rectangle(
        (margin, margin, size - margin - 1, body_bot),
        radius=radius,
        fill=255,
    )
    d.polygon(
        [
            (margin + radius, body_bot - radius // 2),
            (size - margin - radius - 1, body_bot - radius // 2),
            (size // 2, size - margin - 1),
        ],
        fill=255,
    )
    return mask


def gradient(size: int, top, bot) -> Image.Image:
    img = Image.new("RGBA", (size, size), bot)
    d = ImageDraw.Draw(img)
    for y in range(size):
        t = y / max(1, size - 1)
        r = int(top[0] * (1 - t) + bot[0] * t)
        g = int(top[1] * (1 - t) + bot[1] * t)
        b = int(top[2] * (1 - t) + bot[2] * t)
        d.line([(0, y), (size, y)], fill=(r, g, b, 255))
    return img


def load_font(size: int) -> ImageFont.FreeTypeFont:
    """Load a bold system font.  Falls back to a default if
    no bold font is found."""
    candidates = [
        "/System/Library/Fonts/Supplemental/Verdana Bold.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ]
    for path in candidates:
        try:
            return ImageFont.truetype(path, size)
        except (OSError, FileNotFoundError):
            continue
    return ImageFont.load_default()


def make_icon(size: int) -> Image.Image:
    grad = gradient(size, TEAL, TEAL_DARK)
    mask = shield_mask(size)
    shield = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shield.paste(grad, (0, 0), mask)

    # Top-edge highlight.
    hl = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    hd = ImageDraw.Draw(hl)
    hd.rectangle(
        (0, 0, size - 1, int(size * 0.04)),
        fill=HIGHLIGHT,
    )
    hl = hl.filter(ImageFilter.GaussianBlur(radius=size * 0.02))
    shield = Image.alpha_composite(shield, hl)

    # The K.  Use a bold system font sized so the K
    # occupies about 55% of the canvas width, centered
    # vertically in the upper body.
    font_size = int(size * 0.66)
    font = load_font(font_size)
    d = ImageDraw.Draw(shield)
    text = "K"
    bbox = d.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    # Center horizontally, and vertically in the body
    # (which is the top 72% of the canvas).
    body_top = int(size * 0.02)
    body_bot = int(size * 0.72)
    body_cy = (body_top + body_bot) // 2
    # bbox may have negative top (font ascent); use it to
    # compute a draw position that visually centers the K.
    tx = (size - tw) // 2 - bbox[0]
    ty = body_cy - th // 2 - bbox[1]
    d.text((tx, ty), text, font=font, fill=WHITE)
    return shield


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
