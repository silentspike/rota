#!/usr/bin/env python3
"""Bereitet die erzeugten Rahmen fuer die Auslieferung auf (Issue #22).

Die Erzeugung liefert grosse PNG mit Alphakanal, aber nicht buendig: Der Rahmen
sitzt nicht exakt am Bildrand, und die Dateien sind fuer ein Browserspiel viel
zu schwer (bis 1,2 MB je Bild).

Drei Schritte, alle messbar:

  1. ZUSCHNEIDEN   auf das, was tatsaechlich sichtbar ist (Alpha > 0)
  2. VERKLEINERN   auf die doppelte Zielgroesse im HUD - doppelt, damit die
                   Kanten auf hochaufloesenden Bildschirmen scharf bleiben
  3. AUSDUENNEN    Farbtiefe reduzieren; der Rahmen ist fast farblos, also
                   kostet eine Palette nichts an Aussehen, aber viel an Groesse

Die Zielgroessen stammen aus dem laufenden Code, nicht aus einer Annahme:
src/render/spellbar.ts (SLOT_SIZE) und src/render/vessel.ts (VESSEL_RADIUS,
VESSEL_THICKNESS).

Fuer die Instrumententafel gibt es KEIN eigenes Bild. Sie benutzt denselben
slot-frame.png als 9-Slice (CSS border-image): Ecken bleiben unveraendert, nur
die Kanten werden gestreckt - und zwar laengs, nie quer. Damit ist die
Bandstaerke unabhaengig von der Panelgroesse ueberall gleich. Ein gestrecktes
Vollbild kann das nicht: Beim ersten Versuch frass das Band die Werte in der
Tafel auf, weil gleiche RELATIVE Bandbreite nicht gleiche absolute Staerke
bedeutet.

Aufruf:

    python3 tools/frame-prepare.py --from gen-tmp
"""

from __future__ import annotations

import argparse
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(REPO, "assets", "img", "frames")

# Zielgroessen in Entwurfs-Koordinaten (1920 x 1080), verdoppelt fuer scharfe
# Kanten auf hochaufloesenden Bildschirmen.
TARGETS = {
    # Zauber-Symbol-Slot: SLOT_SIZE = 104 (src/render/spellbar.ts)
    "slot-frame.png": (208, 208),
    # Reaktionsgefaess: VESSEL_RADIUS 186 + VESSEL_THICKNESS 34 / 2 = 203,
    # verdoppelt. Eine FASSUNG aus zwei schmalen Reifen - der Kanal dazwischen
    # bleibt frei, weil dort der Fuellstand laeuft (Dekor darf Information nie
    # ueberlagern).
    "ring-mount.png": (812, 812),
}


def trim(img):
    """Schneidet auf den sichtbaren Inhalt zu."""
    box = img.getchannel("A").point(lambda v: 255 if v > 0 else 0).getbbox()
    return img.crop(box) if box else img


def main(argv: list[str]) -> int:
    from PIL import Image, ImageOps

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--from", dest="src", default=os.path.join(REPO, "gen-tmp"))
    parser.add_argument("--out", default=OUT)
    args = parser.parse_args(argv[1:])

    os.makedirs(args.out, exist_ok=True)
    print(f"{'Datei':<18}{'erzeugt':>12}{'zugeschnitten':>16}{'geliefert':>12}{'kB vorher':>11}{'kB nachher':>12}")
    print("-" * 82)

    ok = True
    for name, (tw, th) in TARGETS.items():
        src = os.path.join(args.src, name)
        if not os.path.exists(src):
            print(f"{name:<18}FEHLT unter {args.src}")
            ok = False
            continue
        before_kb = os.path.getsize(src) / 1024

        img = Image.open(src).convert("RGBA")
        raw = img.size
        img = trim(img)
        cropped = img.size
        img = img.resize((tw, th), Image.LANCZOS)

        dst = os.path.join(args.out, name)
        # Der Rahmen ist nahezu farblos, die Farbkanaele vertragen also eine
        # grobe Abstufung. Der ALPHAKANAL bleibt dabei unberuehrt: Eine Palette
        # ueber alle vier Kanaele (quantize) quantisiert auch die Deckung, und
        # danach ist praktisch kein Pixel mehr voll deckend - freigestellt waere
        # das Element dann nur noch auf dem Papier.
        r, g, b, alpha = img.split()
        rgb = Image.merge("RGB", (r, g, b))
        posterized = ImageOps.posterize(rgb, 5)
        Image.merge("RGBA", (*posterized.split(), alpha)).save(dst, optimize=True)
        after_kb = os.path.getsize(dst) / 1024

        print(
            f"{name:<18}{f'{raw[0]}x{raw[1]}':>12}{f'{cropped[0]}x{cropped[1]}':>16}"
            f"{f'{tw}x{th}':>12}{before_kb:11.0f}{after_kb:12.0f}"
        )

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
