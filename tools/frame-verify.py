#!/usr/bin/env python3
"""Prueft die erzeugten Rahmen gegen die Acceptance Criteria von Issue #22.

  AC1  Jedes Element liegt einzeln und freigestellt vor
  AC3  Die Lichtrichtung ist ueber alle Elemente gleich

AC2 (keine sichtbaren Kanten ueber dem Geruest) und der Augenschein zu AC3
gehoeren in den Browser - siehe docs/frame-check/.

WIE DIE LICHTRICHTUNG GEMESSEN WIRD
-----------------------------------
Ueber die deckenden Pixel wird die mittlere Helligkeit in Dritteln verglichen:
oben gegen unten, links gegen rechts. Licht von oben links heisst, dass beide
Differenzen deutlich positiv sind.

"Deutlich" ist keine Ansichtssache: Beim ersten Versuch lagen die Werte bei
+6,0 und +0,4 von 255 - das ist Rauschen. Wo keine Richtung ist, laesst sich
auch keine Einheitlichkeit belegen. Verlangt sind deshalb mindestens 15
Helligkeitsstufen je Achse, und alle Elemente muessen dasselbe Vorzeichen
tragen.

Aufruf:

    python3 tools/frame-verify.py
"""

from __future__ import annotations

import argparse
import glob
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FRAMES = os.path.join(REPO, "assets", "img", "frames")

# Mindest-Helligkeitsunterschied je Achse, in Stufen von 255.
MIN_GRADIENT = 15.0


def light(path: str) -> tuple[float, float, float, float, float]:
    """Liefert (oben-unten, links-rechts, transparent, halbtransparent, deckend)."""
    import numpy as np
    from PIL import Image

    a = np.asarray(Image.open(path).convert("RGBA"))
    alpha = a[..., 3] / 255.0
    rgb = a[..., :3].astype(float).mean(axis=2)
    h, w = alpha.shape
    solid = alpha > 0.9

    def mean(region_rgb, region_mask):
        return float(region_rgb[region_mask].mean()) if region_mask.any() else 0.0

    top = mean(rgb[: h // 3], solid[: h // 3])
    bottom = mean(rgb[2 * h // 3 :], solid[2 * h // 3 :])
    left = mean(rgb[:, : w // 3], solid[:, : w // 3])
    right = mean(rgb[:, 2 * w // 3 :], solid[:, 2 * w // 3 :])
    return (
        top - bottom,
        left - right,
        float((alpha == 0).mean()),
        float(((alpha > 0) & (alpha < 1)).mean()),
        float((alpha == 1).mean()),
    )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frames", default=FRAMES)
    args = parser.parse_args(argv[1:])

    files = sorted(glob.glob(os.path.join(args.frames, "*.png")))
    if not files:
        print(f"Keine Rahmen unter {args.frames}")
        return 1

    print(f"{len(files)} Elemente\n")
    print(f"{'Datei':<20}{'transp.':>9}{'halbtr.':>9}{'deckend':>9}{'oben-unten':>12}{'links-rechts':>14}")
    print("-" * 73)

    fail1: list[str] = []
    fail3: list[str] = []
    vertical: list[float] = []
    horizontal: list[float] = []

    for path in files:
        dy, dx, transparent, partial, solid = light(path)
        rel = os.path.basename(path)
        vertical.append(dy)
        horizontal.append(dx)

        # AC1: freigestellt heisst, es gibt ueberhaupt durchsichtige Flaeche -
        # ein Element auf undurchsichtigem Grund waere kein Einzelteil.
        if transparent < 0.10:
            fail1.append(f"{rel}: nur {transparent * 100:.1f} % transparent, nicht freigestellt")
        if solid < 0.02:
            fail1.append(f"{rel}: fast nichts Deckendes, das Element fehlt")

        if abs(dy) < MIN_GRADIENT or abs(dx) < MIN_GRADIENT:
            fail3.append(
                f"{rel}: Lichtrichtung zu schwach ({dy:+.1f} / {dx:+.1f}, "
                f"verlangt sind {MIN_GRADIENT:.0f})"
            )

        print(
            f"{rel:<20}{transparent * 100:8.1f}%{partial * 100:8.2f}%{solid * 100:8.1f}%"
            f"{dy:+12.1f}{dx:+14.1f}"
        )

    # Alle Elemente muessen aus derselben Richtung beleuchtet sein.
    if len({dy > 0 for dy in vertical}) > 1:
        fail3.append("die senkrechte Lichtrichtung ist nicht bei allen gleich")
    if len({dx > 0 for dx in horizontal}) > 1:
        fail3.append("die waagerechte Lichtrichtung ist nicht bei allen gleich")

    print("-" * 73)
    for label, problems in (
        ("AC1  jedes Element einzeln und freigestellt", fail1),
        ("AC3  Lichtrichtung ueber alle Elemente gleich", fail3),
    ):
        if problems:
            print(f"{label}: NICHT BESTANDEN")
            for p in problems:
                print(f"       {p}")
        else:
            print(f"{label}: BESTANDEN")

    if not fail3:
        richtung = "oben links" if vertical[0] > 0 and horizontal[0] > 0 else "uneindeutig"
        print(
            f"\nLicht kommt bei allen {len(files)} Elementen von {richtung}; "
            f"schwaechster Wert {min(min(map(abs, vertical)), min(map(abs, horizontal))):.1f} Stufen"
        )
    return 1 if (fail1 or fail3) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
