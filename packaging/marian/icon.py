#!/usr/bin/env python3
"""Marian DesignCraft's app icon: a cream pilcrow (Source Serif 4 Bold, OFL, from assets/fonts)
on a darker red rounded square. Writes assets/app-icon/marian-designcraft-*.png and .icns.
Needs Pillow. Run from anywhere: python3 packaging/marian/icon.py"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/app-icon"
RED = (160, 27, 32)
CREAM = (250, 244, 236)
S = 1024


def tile(size, inset):
    """The icon at `size` px, its square `inset` px in from each edge (macOS leaves a margin)."""
    big = S
    im = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    side = big - 2 * inset
    d.rounded_rectangle([inset, inset, big - inset - 1, big - inset - 1], radius=int(side * 0.2237), fill=RED + (255,))
    font = ImageFont.truetype(str(ROOT / "assets/fonts/SourceSerif4-Bold.ttf"), int(side * 0.74))
    b = d.textbbox((0, 0), "¶", font=font)
    w, h = b[2] - b[0], b[3] - b[1]
    d.text(((big - w) / 2 - b[0], (big - h) / 2 - b[1] + side * 0.01), "¶", font=font, fill=CREAM)
    return im.resize((size, size), Image.LANCZOS)


tile(1024, 0).save(OUT / "marian-designcraft-1024.png")
tile(256, 0).save(OUT / "marian-designcraft-256.png")
mac = tile(1024, 100)
mac.resize((512, 512), Image.LANCZOS).save(OUT / "marian-designcraft-macos-512.png")
mac.save(OUT / "marian-designcraft.icns", sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)])
print("wrote", *sorted(p.name for p in OUT.glob("marian-designcraft*")))
