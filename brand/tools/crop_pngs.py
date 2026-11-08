#!/usr/bin/env python3
"""Crop staged 1600x1600 screenshots to the exact target PNGs and build the .ico."""
from __future__ import annotations

import json
import math
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "brand"
TOOLS = BRAND / "tools"
STAGE = TOOLS / ".stage"

jobs = json.loads((TOOLS / "render_manifest.json").read_text(encoding="utf-8"))
for job in jobs:
    staged = STAGE / job["out"]
    target = BRAND / job["out"]
    target.parent.mkdir(parents=True, exist_ok=True)
    w = math.ceil(float(job["w"]))
    h = math.ceil(float(job["h"]))
    with Image.open(staged) as im:
        im = im.convert("RGBA").crop((0, 0, w, h))
        im.save(target, optimize=True)
    print(f"{job['out']}: {w}x{h}")

ico_src = BRAND / "png" / "app-icon-1024.png"
ico_out = BRAND / "app-icon.ico"
with Image.open(ico_src) as im:
    im = im.convert("RGBA")
    im.save(ico_out, sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print(f"wrote {ico_out.name}")
