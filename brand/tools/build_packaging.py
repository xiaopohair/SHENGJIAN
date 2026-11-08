#!/usr/bin/env python3
"""Compose the NSIS installer artwork (header + sidebar) from brand assets.

Usage: python brand/tools/build_packaging.py
Writes: brand/packaging/installer-header.{png,bmp}
        brand/packaging/installer-sidebar.{png,bmp}

NSIS wants 24-bit BMP; PNG copies are kept for review. The artwork follows
BRAND.md: ink background, one focus per asset, teal mark from the brand
assets, no photos / gradients / glow.
"""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "brand"
PNG = BRAND / "png"
OUT = BRAND / "packaging"

INK = (0x0B, 0x0F, 0x14)
PAPER = (0xF2, 0xF5, 0xF4)
MIST = (0x8B, 0x99, 0xAC)
TEAL = (0x5A, 0xC8, 0xAA)
DEEP = (0x2E, 0x8F, 0x76)

FONT_CANDIDATES = [
    Path(r"C:\Windows\Fonts\msyh.ttc"),
    Path(r"C:\Windows\Fonts\msyhl.ttc"),
]


def font(size: int) -> ImageFont.FreeTypeFont:
    for path in FONT_CANDIDATES:
        if path.exists():
            return ImageFont.truetype(str(path), size)
    raise SystemExit("Microsoft YaHei UI font not found; install msyh.ttc or pin a font path")


def paste_scaled(canvas: Image.Image, source: Path, height: int, xy: tuple[int, int]) -> int:
    with Image.open(source) as im:
        im = im.convert("RGBA")
        width = round(im.width * height / im.height)
        scaled = im.resize((width, height), Image.LANCZOS)
    canvas.alpha_composite(scaled, xy)
    return width


def header() -> None:
    canvas = Image.new("RGBA", (150, 57), INK + (255,))
    # One focus: mark + wordmark, optically centred.
    with Image.open(PNG / "mark-64.png") as mark, Image.open(PNG / "wordmark-light-2x.png") as wm:
        mark_h = 38
        mark_w = round(mark.width * mark_h / mark.height)
        wm_h = 26
        wm_w = round(wm.width * wm_h / wm.height)
        gap = 8
        total = mark_w + gap + wm_w
        x0 = round((150 - total) / 2)
        paste_scaled(canvas, PNG / "mark-64.png", mark_h, (x0, round((57 - mark_h) / 2)))
        paste_scaled(canvas, PNG / "wordmark-light-2x.png", wm_h, (x0 + mark_w + gap, round((57 - wm_h) / 2)))
    canvas.convert("RGB").save(OUT / "installer-header.bmp", "BMP")
    canvas.save(OUT / "installer-header.png")


def sidebar() -> None:
    canvas = Image.new("RGBA", (164, 314), INK + (255,))
    draw = ImageDraw.Draw(canvas)

    # Stacked lockup for dark backgrounds (teal mark + snow wordmark).
    with Image.open(PNG / "lockup-stacked-dark-2x.png") as lockup:
        width = 116
        height = round(lockup.height * width / lockup.width)
        scaled = lockup.convert("RGBA").resize((width, height), Image.LANCZOS)
    canvas.alpha_composite(scaled, (round((164 - width) / 2), 64))

    # Thin teal rule, then the slogan in the UI font (界面字体).
    rule_y = 250
    draw.rectangle((56, rule_y, 108, rule_y + 2), fill=TEAL)

    slogan = "声音，就是快捷键。"
    label_font = font(10)
    box = draw.textbbox((0, 0), slogan, font=label_font)
    text_w = box[2] - box[0]
    draw.text(((164 - text_w) / 2 - box[0], 264), slogan, font=label_font, fill=PAPER)

    english = "YOUR VOICE IS THE SHORTCUT"
    en_font = font(6)
    box = draw.textbbox((0, 0), english, font=en_font)
    text_w = box[2] - box[0]
    draw.text(((164 - text_w) / 2 - box[0], 280), english, font=en_font, fill=MIST)

    canvas.convert("RGB").save(OUT / "installer-sidebar.bmp", "BMP")
    canvas.save(OUT / "installer-sidebar.png")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    header()
    sidebar()
    print("wrote installer-header.bmp and installer-sidebar.bmp")


if __name__ == "__main__":
    main()
