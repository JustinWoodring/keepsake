#!/usr/bin/env python3
"""Generate the Keepsake bundle icons from the source art.

The source image is at `assets/source-icon.png` (an isometric
vault building with a slate roof, teal windows, and a brass
shield emblem).  This script crops it to a square, scales it
to the sizes Tauri's bundler expects, and writes:

  - PNG files for Linux AppImage, Linux .deb, macOS app
    (alongside the .icns), and Windows .ico-source.
  - A multi-resolution icon.ico for Windows.
  - A multi-resolution icon.icns for macOS (Tauri's
    macOS bundler is strict and refuses .png/.ico).

To re-generate from a new source:

  1. Drop the new image at `assets/source-icon.png`.
  2. Run `python3 scripts/make-icons.py`.

For CI runners where system Python is externally managed
(PEP 668), pass `--in-venv <path>` to install Pillow into a
throwaway venv first:

  python3 scripts/make-icons.py --in-venv .venv-icons

That flag is handled entirely inside Python so the call
works identically on POSIX shells and PowerShell.
"""
from __future__ import annotations

# IMPORTANT: this bootstrap block must run before any
# `from PIL import Image` (or any other import that requires
# Pillow).  We do the venv re-exec *before* importing
# Pillow so the re-exec'd process is the one that ends up
# importing it.  See `bootstrap_venv()`.
import os
import subprocess
import sys
from pathlib import Path


def venv_python(venv_dir: Path) -> Path:
    """Locate the python interpreter inside a venv created
    with `python -m venv`.  Portable across POSIX
    ('bin/python') and Windows ('Scripts/python.exe')."""
    if os.name == "nt":
        return venv_dir / "Scripts" / "python.exe"
    return venv_dir / "bin" / "python"


def ensure_venv(venv_dir: Path) -> Path:
    """Create a venv if it doesn't exist and install Pillow
    into it.  Returns the venv's python interpreter path."""
    py = venv_python(venv_dir)
    if not py.exists():
        subprocess.check_call(
            [sys.executable, "-m", "venv", str(venv_dir)],
        )
    subprocess.check_call(
        [str(py), "-m", "pip", "install", "--quiet", "Pillow"],
    )
    return py


def parse_in_venv(argv: list[str], root: Path) -> tuple[Path | None, list[str]]:
    """Strip `--in-venv <path>` from argv; return the
    parsed path (resolved against root) and the leftover
    args."""
    in_venv: Path | None = None
    rest: list[str] = []
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--in-venv" and i + 1 < len(argv):
            in_venv = (root / argv[i + 1]).resolve()
            i += 2
            continue
        rest.append(a)
        i += 1
    return in_venv, rest


def bootstrap_venv() -> None:
    """Handle `--in-venv` re-exec before any heavy imports.
    Raises SystemExit after the re-exec; otherwise
    returns and the rest of the script proceeds.

    Identifying "are we already in the venv" is tricky
    because venvs created by `python -m venv` symlink
    python3 back to the parent interpreter.  Comparing
    `sys.executable` resolves through the symlink to the
    same binary in both processes.  Use `sys.prefix`
    instead, which IS isolated — `sys.prefix` is the venv
    directory when inside a venv, or the system prefix
    when not.
    """
    in_venv, rest = parse_in_venv(sys.argv[1:], ROOT)
    if in_venv is None:
        return
    py = ensure_venv(in_venv)
    # sys.prefix is the venv root when we're in a venv.
    if Path(sys.prefix).resolve() == in_venv.resolve():
        sys.stderr.write(f"[make-icons] already in venv: {sys.prefix}\n")
        return
    sys.stderr.write(
        f"[make-icons] re-exec into venv: "
        f"prefix={sys.prefix} -> {in_venv}\n"
    )
    rc = subprocess.call([str(py), __file__, *rest])
    raise SystemExit(rc)


# Run the venv bootstrap *before* importing PIL.  Any
# failure here exits with a clear message instead of a
# confusing ModuleNotFoundError.
ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets" / "source-icon.png"
ICON_DIR = ROOT / "crates" / "keepsake-app" / "src-tauri" / "icons"

try:
    bootstrap_venv()
except SystemExit:
    raise
except Exception as e:
    sys.stderr.write(f"venv bootstrap failed: {e}\n")
    sys.exit(2)

# Now safe to import PIL: either the outer python has it,
# or we just re-exec'd inside the venv that does.
from io import BytesIO  # noqa: E402

import struct  # noqa: E402

from PIL import Image  # noqa: E402

SIZE = 1024


def load_source() -> Image.Image:
    """Load the source art as a centered square RGBA image."""
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
    return img.crop((left, top, left + side, top + side))


def make_icon(size: int, source: Image.Image) -> Image.Image:
    return source.resize((size, size), Image.LANCZOS)


def make_ico(source: Image.Image, out: Path) -> None:
    """Multi-resolution Windows .ico."""
    make_icon(SIZE, source).save(
        out,
        format="ICO",
        sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)],
    )


def make_icns(source: Image.Image, out: Path) -> None:
    """Multi-resolution macOS .icns.  Pillow can't write
    ICNS, so we assemble the file by hand.

    The format is:

        magic   'icns'      (4 bytes)
        total   big-u32     (4 bytes, total file size)
        chunk1  type(4) + size(4) + data
        chunk2  ...
        ...

    For 16x16 / 32x32 ('icp4' / 'icp5') the data is
    premultiplied BGRA.  For 128x128 and up ('ic07'+
    = 'ic07', 'ic08', 'ic09', 'ic10') the data is PNG.

    Reference: Apple TN1194.
    """
    elements_argb = [
        (b"icp4", 16),
        (b"icp5", 32),
    ]
    elements_png = [
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
    ]

    def png_at(size: int) -> bytes:
        buf = BytesIO()
        make_icon(size, source).save(buf, format="PNG", optimize=True)
        return buf.getvalue()

    def argb_at(size: int) -> bytes:
        return make_icon(size, source).tobytes("raw", "BGRA")

    chunks: list[bytes] = []
    for typ, sz in elements_argb:
        data = argb_at(sz)
        chunks.append(typ + struct.pack(">I", 8 + len(data)) + data)
    for typ, sz in elements_png:
        data = png_at(sz)
        chunks.append(typ + struct.pack(">I", 8 + len(data)) + data)
    body = b"".join(chunks)
    icns = b"icns" + struct.pack(">I", 8 + len(body)) + body
    out.write_bytes(icns)


def write_all_icons(source: Image.Image) -> None:
    ICON_DIR.mkdir(parents=True, exist_ok=True)
    full = make_icon(SIZE, source)
    full.save(ICON_DIR / "icon.png")
    full.resize((512, 512), Image.LANCZOS).save(ICON_DIR / "icon-512.png")
    full.resize((256, 256), Image.LANCZOS).save(ICON_DIR / "icon-256.png")
    full.resize((128, 128), Image.LANCZOS).save(ICON_DIR / "icon-128.png")
    full.resize((32, 32), Image.LANCZOS).save(ICON_DIR / "32x32.png")
    full.resize((32, 32), Image.LANCZOS).save(ICON_DIR / "icon-32.png")
    full.resize((128, 128), Image.LANCZOS).save(ICON_DIR / "128x128.png")
    full.resize((256, 256), Image.LANCZOS).save(ICON_DIR / "128x128@2x.png")
    full.resize((384, 384), Image.LANCZOS).save(ICON_DIR / "128x128@3x.png")
    make_ico(source, ICON_DIR / "icon.ico")
    make_icns(source, ICON_DIR / "icon.icns")
    print(f"wrote icons to {ICON_DIR}")


def main() -> int:
    source = load_source()
    write_all_icons(source)
    return 0


if __name__ == "__main__":
    sys.exit(main())