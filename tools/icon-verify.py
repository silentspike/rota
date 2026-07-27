#!/usr/bin/env python3
"""Prueft die Zauber-Symbole gegen die Acceptance Criteria von Issue #20.

Jede Pruefung misst. Keine sagt "sieht richtig aus".

  AC1  Alphakanal mit weichen Raendern - halbtransparenter Anteil ueber 2 %
  AC2  quadratisches Feld, Inhalt zentriert
  AC3  keine dunklen Raender auf hellem Grund
  AC4  alle Symbole mit derselben Kantenlaenge

Aufruf:

    python3 tools/icon-verify.py            # Pruefgroesse 72 px (Spielgroesse)
    python3 tools/icon-verify.py --size 128

Rueckgabewert 0, wenn alle vier bestehen, sonst 1.
"""

from __future__ import annotations

import argparse
import glob
import os
import re
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS = os.path.join(REPO, "assets", "img", "icons")

# Die Zauberleiste zeichnet mit SLOT_SIZE = 92 (src/render/hud.ts); das Symbol
# sitzt mit Rand darin.
GAME_SIZE = 72

MIN_PARTIAL = 0.02  # AC1: mindestens 2 % halbtransparente Flaeche
MAX_OFFSET = 0.02  # AC2: Mitte darf hoechstens 2 % der Feldbreite abweichen


def viewbox(path: str) -> tuple[float, float]:
    with open(path, encoding="utf-8") as fh:
        head = fh.read(400)
    m = re.search(r'viewBox="([\d.\- ]+)"', head)
    if not m:
        raise ValueError(f"{path}: keine viewBox")
    _x, _y, w, h = (float(v) for v in m.group(1).split())
    return w, h


def bbox(path: str) -> tuple[float, float, float, float]:
    out = subprocess.run(
        ["inkscape", "--query-all", path], capture_output=True, text=True, check=True
    ).stdout.strip()
    _id, x, y, w, h = out.splitlines()[0].split(",")
    return float(x), float(y), float(w), float(h)


def raster(path: str, size: int, out_png: str) -> None:
    subprocess.run(
        ["inkscape", "-w", str(size), "-h", str(size), path, "-o", out_png],
        check=True,
        capture_output=True,
    )


def main(argv: list[str]) -> int:
    from PIL import Image

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=GAME_SIZE)
    parser.add_argument("--icons", default=ICONS)
    args = parser.parse_args(argv[1:])

    files = sorted(glob.glob(os.path.join(args.icons, "**", "*.svg"), recursive=True))
    if not files:
        print(f"Keine Symbole unter {args.icons}")
        return 1

    print(f"{len(files)} Symbole, Pruefgroesse {args.size} px\n")
    print(f"{'Datei':<32}{'viewBox':>12}{'halbtr.':>9}{'Mitte':>9}{'dunkel':>8}{'Kante':>8}")
    print("-" * 78)

    fail1: list[str] = []
    fail2: list[str] = []
    fail3: list[str] = []
    fail4: list[str] = []
    sizes: set[tuple[int, int]] = set()
    boxes: set[tuple[float, float]] = set()

    with tempfile.TemporaryDirectory() as tmpdir:
        for path in files:
            rel = os.path.relpath(path, args.icons)
            vw, vh = viewbox(path)
            boxes.add((vw, vh))

            png = os.path.join(tmpdir, rel.replace(os.sep, "_") + ".png")
            raster(path, args.size, png)
            img = Image.open(png).convert("RGBA")
            sizes.add(img.size)

            # --- AC1: weiche Kanten
            hist = img.getchannel("A").histogram()
            total = img.size[0] * img.size[1]
            partial = (total - hist[0] - hist[255]) / total
            if partial < MIN_PARTIAL:
                fail1.append(f"{rel}: nur {partial * 100:.2f} % halbtransparent")

            # --- AC2: quadratisches Feld, Inhalt zentriert
            square = abs(vw - vh) < 0.5
            bx, by, bw, bh = bbox(path)
            dx = abs((bx + bw / 2) - vw / 2) / vw
            dy = abs((by + bh / 2) - vh / 2) / vh
            off = max(dx, dy)
            if not square:
                fail2.append(f"{rel}: viewBox {vw:.0f} x {vh:.0f} ist nicht quadratisch")
            elif off > MAX_OFFSET:
                fail2.append(f"{rel}: Inhalt {off * 100:.1f} % aus der Mitte")

            # --- AC3: keine dunklen Raender
            # Ein Rand ist dunkel, wenn ein teilweise deckender Pixel eine
            # dunklere Farbe traegt als der Kern. Bei sauberen Vektorpfaden
            # ist die Farbe ueberall dieselbe; ein Restschatten aus einem
            # PNG-Freisteller waere hier sofort sichtbar.
            px = img.load()
            darkest = 255
            for yy in range(img.size[1]):
                for xx in range(img.size[0]):
                    r, g, b, a = px[xx, yy]
                    if a > 0:
                        darkest = min(darkest, max(r, g, b))
            if darkest < 250:
                fail3.append(f"{rel}: dunkelster sichtbarer Wert {darkest}")

            print(
                f"{rel:<32}{f'{vw:.0f}x{vh:.0f}':>12}{partial * 100:8.2f}%"
                f"{off * 100:8.1f}%{darkest:8d}{f'{img.size[0]}':>8}"
            )

    # --- AC4: gleiche Kantenlaenge
    if len(sizes) != 1:
        fail4.append(f"verschiedene Rastergroessen: {sorted(sizes)}")
    if len(boxes) != 1:
        fail4.append(f"verschiedene viewBox-Groessen: {sorted(boxes)}")

    print("-" * 78)
    results = [
        ("AC1  Alphakanal mit weichen Raendern (ueber 2 %)", fail1),
        ("AC2  quadratisches Feld, Inhalt zentriert", fail2),
        ("AC3  keine dunklen Raender auf hellem Grund", fail3),
        ("AC4  alle Symbole mit derselben Kantenlaenge", fail4),
    ]
    ok = True
    for label, problems in results:
        if problems:
            ok = False
            print(f"{label}: NICHT BESTANDEN")
            for p in problems[:10]:
                print(f"       {p}")
            if len(problems) > 10:
                print(f"       ... und {len(problems) - 10} weitere")
        else:
            print(f"{label}: BESTANDEN")

    if ok:
        vw, vh = next(iter(boxes))
        w, h = next(iter(sizes))
        print(f"\nAlle {len(files)} Symbole: viewBox {vw:.0f} x {vh:.0f}, gerastert {w} x {h}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
