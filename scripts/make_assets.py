#!/usr/bin/env python3
"""Generate app icons, tray icons and the completion chime."""

import math
import struct
import wave
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"
ASSETS = ROOT / "src-tauri" / "assets"
ICONS.mkdir(parents=True, exist_ok=True)
ASSETS.mkdir(parents=True, exist_ok=True)

BG = (19, 20, 24, 255)
SURFACE = (32, 34, 40, 255)
ACCENT = (90, 156, 255, 255)
MINT = (124, 231, 196, 255)
TEXT = (244, 246, 250, 255)
MUTED = (170, 175, 186, 255)


def app_icon() -> None:
    s = 1024
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    d.rounded_rectangle((44, 44, s - 44, s - 44), radius=224, fill=BG)
    d.rounded_rectangle((56, 56, s - 56, s - 56), radius=214, outline=(255, 255, 255, 20), width=5)

    cx = cy = s / 2
    inset = 214
    box = (inset, inset, s - inset, s - inset)
    r = (s - inset * 2) / 2
    ring_w = 54

    d.ellipse(box, outline=(255, 255, 255, 34), width=ring_w)
    d.arc(box, start=-90, end=208, fill=ACCENT, width=ring_w)
    for ang in (-90, 208):
        x = cx + r * math.cos(math.radians(ang))
        y = cy + r * math.sin(math.radians(ang))
        d.ellipse((x - ring_w / 2, y - ring_w / 2, x + ring_w / 2, y + ring_w / 2), fill=ACCENT)

    d.line((cx, cy, cx, cy - 178), fill=TEXT, width=34)
    d.line((cx, cy, cx + 136, cy + 78), fill=MINT, width=26)
    d.ellipse((cx - 28, cy - 28, cx + 28, cy + 28), fill=TEXT)

    img.save(ICONS / "icon-source.png")
    for name, size in {
        "icon.png": 512,
        "128x128@2x.png": 256,
        "128x128.png": 128,
        "32x32.png": 32,
    }.items():
        img.resize((size, size), Image.LANCZOS).save(ICONS / name)


def tray_icon(name: str, color: tuple[int, int, int, int]) -> None:
    s = 256
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    box = (30, 30, s - 30, s - 30)
    d.ellipse(box, outline=color, width=20)
    cx = cy = s / 2
    d.line((cx, cy, cx, cy - 72), fill=color, width=18)
    d.line((cx, cy, cx + 52, cy + 30), fill=color, width=18)
    d.ellipse((cx - 13, cy - 13, cx + 13, cy + 13), fill=color)
    img.resize((32, 32), Image.LANCZOS).save(ICONS / f"{name}.png")


def chime() -> None:
    rate = 44100
    duration = 1.6
    frames = bytearray()
    total = int(rate * duration)
    for i in range(total):
        t = i / rate
        env = math.exp(-3.2 * t)
        # Two soft notes: E6 then A6, with a little shimmer.
        tone = (
            0.55 * math.sin(2 * math.pi * 1318.51 * t)
            + 0.35 * math.sin(2 * math.pi * 1760.0 * t)
            + 0.12 * math.sin(2 * math.pi * 2637.02 * t)
        )
        click = 0.0
        if t < 0.02:
            click = 0.3 * math.sin(2 * math.pi * 880 * t) * (1 - t / 0.02)
        sample = max(-1.0, min(1.0, (tone * env + click) * 0.55))
        frames += struct.pack("<h", int(sample * 32767))

    with wave.open(str(ASSETS / "chime.wav"), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(rate)
        f.writeframes(bytes(frames))


if __name__ == "__main__":
    app_icon()
    tray_icon("tray-idle", MUTED)
    tray_icon("tray-active", ACCENT)
    chime()
    print("assets written")
