#!/usr/bin/env python3
"""Bringt die Zauber-Symbole auf ein einheitliches Feld und ein einheitliches
optisches Gewicht (Issue #20, Stil GEAETZT).

WARUM DIESES WERKZEUG ANDERS AUSSIEHT, ALS DAS ISSUE ES BESCHREIBT
------------------------------------------------------------------
Issue #20 wurde fuer erzeugte PNG-Symbole geschrieben: Alphakanal pruefen,
Farbreste ausbluten lassen, zuschneiden. Mit der Stil-Entscheidung GEAETZT
(GAME-DESIGN 17.2) kommen die Symbole als Vektor von game-icons.net, und damit
faellt ein Teil dieser Arbeit ersatzlos weg:

  - Ein SVG-Pfad hat keinen Farbrest hinter der Transparenz. Ausbluten
    entfaellt, weil das Problem nicht existiert.
  - Weiche Kanten entstehen beim Rastern von selbst (gemessen: 7-24 % der
    Flaeche sind halbtransparent, verlangt waren ueber 2 %).
  - Die viewBox ist bei allen Symbolen 512 x 512, also bereits quadratisch und
    fuer alle gleich.

Was NICHT von selbst stimmt, ist das optische Gewicht. Gemessen ueber die 24
Symbole in Spielgroesse (72 px): Der Deckungsgrad reicht von 12,6 % bis 56,1 %
- Faktor 4,5. Nebeneinander in der Zauberleiste wirken die einen zierlich, die
anderen fett. Bei Entscheidungen im Millisekundenbereich ist das genau die Art
Unruhe, die stoert.

Dieses Werkzeug gleicht das aus:

  1. Inhalts-Rechteck messen (Inkscape, nicht geschaetzt)
  2. Inhalt exakt in die Mitte eines quadratischen Feldes setzen
  3. So skalieren, dass der Deckungsgrad nahe am MEDIAN des Satzes liegt

Der Median ist bewusst gewaehlt und keine ausgedachte Zahl: Der Satz selbst
bestimmt sein Mass, und die Haelfte der Symbole bewegt sich kaum. Die
Skalierung ist nach oben und unten begrenzt, damit kein Symbol den Rahmen
sprengt oder zur Briefmarke wird.

Aufruf:

    python3 tools/icon-normalize.py                    # misst und schreibt
    python3 tools/icon-normalize.py --check            # misst nur, schreibt nichts
    python3 tools/icon-normalize.py --size 72          # andere Pruefgroesse
"""

from __future__ import annotations

import argparse
import glob
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS = os.path.join(REPO, "assets", "img", "icons")

# Die Zauberleiste zeichnet mit SLOT_SIZE = 92 in Entwurfs-Koordinaten
# (src/render/hud.ts). Das Symbol sitzt mit Rand darin.
GAME_SIZE = 72

# Kantenlaenge der einheitlichen viewBox. game-icons.net liefert 512.
FIELD = 512

# Der Inhalt beruehrt den Feldrand nie ganz - sonst klebt das Symbol im Rahmen.
MARGIN = 0.06

# Grenzen fuer den Gewichtsausgleich, als Anteil der nutzbaren Feldbreite.
# Ohne sie wuerde ein sehr leichtes Symbol ueber den Rand hinaus vergroessert
# und ein sehr schweres auf Briefmarkengroesse geschrumpft.
SCALE_MIN = 0.62
SCALE_MAX = 1.0

# Ziel-Deckungsgrad, FESTGESCHRIEBEN.
#
# Anfangs war das der Median des jeweiligen Satzes. Das war ein Fehler: Wird ein
# Symbol ausgetauscht, wandert der Median, alle uebrigen Symbole werden neu
# skaliert - und die Aehnlichkeitswerte aus Issue #21 springen mit. Ein Tausch,
# der ein Paar entschaerfen soll, riss dadurch an anderer Stelle ein neues auf.
#
# Der Wert stammt aus dem Median der 24 Symbole zum Zeitpunkt von Issue #20 und
# steht seitdem fest. Damit ist die Normalisierung wiederholbar (ein zweiter
# Lauf aendert nichts) und ein Tausch wirkt nur auf das getauschte Symbol.
# Neu bestimmen laesst er sich mit --from-set, aber das ist eine Entscheidung
# und kein Nebeneffekt.
TARGET_INK = 0.31


NORMALIZED_GROUP = re.compile(r'^<g data-rota-normalized="1"[^>]*>(.*)</g>$', re.S)


def strip_normalization(path: str, dst: str) -> str:
    """Schreibt das Symbol OHNE eine fruehere Normalisierung nach dst.

    Alles Weitere wird von dieser Fassung abgeleitet - Inhalts-Rechteck wie
    Skalierung. Wuerde stattdessen die bereits normalisierte Datei vermessen,
    faende die Suche bei jedem Lauf leicht andere Zahlen: gleiches Aussehen,
    aber jedes Mal ein Diff. Wiederholbar ist das nicht.
    """
    with open(path, encoding="utf-8") as fh:
        svg = fh.read()
    inner = re.sub(r"^.*?<svg[^>]*>", "", svg, count=1, flags=re.S)
    inner = re.sub(r"</svg>\s*$", "", inner, flags=re.S).strip()
    m = NORMALIZED_GROUP.match(inner)
    if m:
        inner = m.group(1).strip()
    with open(dst, "w", encoding="utf-8") as fh:
        fh.write(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {FIELD} {FIELD}">{inner}</svg>\n')
    return dst


def query_bbox(path: str) -> tuple[float, float, float, float]:
    """Das Rechteck, das der sichtbare Inhalt tatsaechlich einnimmt."""
    out = subprocess.run(
        ["inkscape", "--query-all", path], capture_output=True, text=True, check=True
    ).stdout.strip()
    _id, x, y, w, h = out.splitlines()[0].split(",")
    return float(x), float(y), float(w), float(h)


def ink_ratio(path: str, size: int) -> float:
    """Deckungsgrad in Spielgroesse: Anteil der Flaeche, den das Symbol faerbt.

    Halbtransparente Pixel zaehlen anteilig - sie sind der Grund, warum ein
    filigranes Symbol optisch leichter wirkt als ein flaechiges gleicher Groesse.
    """
    from PIL import Image

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        png = tmp.name
    try:
        subprocess.run(
            ["inkscape", "-w", str(size), "-h", str(size), path, "-o", png],
            check=True,
            capture_output=True,
        )
        hist = Image.open(png).convert("RGBA").getchannel("A").histogram()
        return sum(i * n for i, n in enumerate(hist)) / (255 * size * size)
    finally:
        os.unlink(png)


def rewrite(src: str, dst: str, bbox: tuple[float, float, float, float], scale: float) -> None:
    """Schreibt das Symbol mittig und skaliert in ein Feld von FIELD x FIELD.

    Der Pfad selbst wird nicht angefasst - er bekommt nur eine Transformation
    davor. Damit bleibt die Zeichnung exakt die des Urhebers, und die Aenderung
    ist jederzeit nachvollziehbar.
    """
    x, y, w, h = bbox
    usable = FIELD * (1 - 2 * MARGIN)
    factor = (usable / max(w, h)) * scale
    # Erst den Inhalt in seinen eigenen Nullpunkt holen, dann skalieren, dann
    # in die Feldmitte schieben.
    tx = FIELD / 2 - (x + w / 2) * factor
    ty = FIELD / 2 - (y + h / 2) * factor

    with open(src, encoding="utf-8") as fh:
        svg = fh.read()

    inner = re.sub(r"^.*?<svg[^>]*>", "", svg, count=1, flags=re.S)
    inner = re.sub(r"</svg>\s*$", "", inner, flags=re.S).strip()

    # Eine fruehere Normalisierung wird ERSETZT, nicht umwickelt. Ohne das
    # schachtelt jeder Lauf eine weitere Transformation ein, die Datei waechst,
    # und das Werkzeug ist nicht mehr wiederholbar - was Issue #21 ausdruecklich
    # verlangt. Erkennbar ist die eigene Gruppe am Merkmal data-rota-normalized.
    stripped = re.match(r'^<g data-rota-normalized="1"[^>]*>(.*)</g>$', inner, flags=re.S)
    if stripped:
        inner = stripped.group(1).strip()

    out = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {FIELD} {FIELD}">'
        f'<g data-rota-normalized="1" transform="translate({tx:.3f} {ty:.3f}) scale({factor:.6f})">'
        f"{inner}"
        f"</g></svg>"
    )
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    with open(dst, "w", encoding="utf-8") as fh:
        fh.write(out + "\n")


def fit_scale(
    src: str, bbox: tuple[float, float, float, float], target: float, size: int
) -> tuple[float, float]:
    """Sucht die Skalierung, deren Deckungsgrad am naechsten am Ziel liegt.

    Binaere Suche ueber acht Schritte. Die Deckung waechst nicht linear mit der
    Skalierung (duenne Linien verlieren beim Verkleinern ueberproportional),
    deshalb wird jeder Schritt gemessen statt gerechnet.
    """
    lo, hi = SCALE_MIN, SCALE_MAX
    best = (SCALE_MAX, 0.0)
    with tempfile.TemporaryDirectory() as tmpdir:
        probe = os.path.join(tmpdir, "probe.svg")
        best_delta = float("inf")
        for _ in range(8):
            mid = (lo + hi) / 2
            rewrite(src, probe, bbox, mid)
            got = ink_ratio(probe, size)
            delta = abs(got - target)
            if delta < best_delta:
                best_delta, best = delta, (mid, got)
            if got > target:
                hi = mid
            else:
                lo = mid
    return best


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="nur messen, nichts schreiben")
    parser.add_argument("--size", type=int, default=GAME_SIZE, help="Pruefgroesse in px")
    parser.add_argument("--icons", default=ICONS, help="Verzeichnis der Symbole")
    parser.add_argument(
        "--target", type=float, default=TARGET_INK, help="Ziel-Deckungsgrad (Vorgabe festgeschrieben)"
    )
    parser.add_argument(
        "--from-set",
        action="store_true",
        help="Ziel neu aus dem Median des Satzes bestimmen statt der festen Vorgabe",
    )
    args = parser.parse_args(argv[1:])

    files = sorted(glob.glob(os.path.join(args.icons, "**", "*.svg"), recursive=True))
    if not files:
        print(f"Keine Symbole unter {args.icons}")
        return 1

    print(f"{len(files)} Symbole, Pruefgroesse {args.size} px\n")

    print("VORHER")
    before = []
    tmp = tempfile.mkdtemp(prefix="rota-icons-")
    try:
        for path in files:
            ratio = ink_ratio(path, args.size)
            # Alles Weitere leitet sich vom Original ab, nicht vom bereits
            # normalisierten Zwischenstand - sonst ist der Lauf nicht wiederholbar.
            base = strip_normalization(path, os.path.join(tmp, os.path.basename(path)))
            before.append((path, ratio, query_bbox(base), base))
            print(f"  {os.path.relpath(path, args.icons):<32}{ratio * 100:6.1f}%")

        ratios = [r for _p, r, _b, _s in before]
        if args.from_set:
            target = statistics.median(ratios)
            quelle = "Median dieses Satzes"
        else:
            target = args.target
            quelle = "festgeschrieben"
        print(
            f"\n  Spanne {min(ratios) * 100:.1f}% bis {max(ratios) * 100:.1f}% "
            f"(Faktor {max(ratios) / min(ratios):.1f}x), Median {statistics.median(ratios) * 100:.1f}%"
        )
        print(f"  Ziel {target * 100:.1f}% ({quelle})")

        if args.check:
            print("\n--check: nichts geschrieben")
            return 0

        print("\nNACHHER")
        after = []
        for path, _ratio, bbox, base in before:
            scale, got = fit_scale(base, bbox, target, args.size)
            rel = os.path.relpath(path, args.icons)
            staged = os.path.join(tmp, "out.svg")
            rewrite(base, staged, bbox, scale)
            shutil.copyfile(staged, path)
            after.append(got)
            print(f"  {rel:<32}{got * 100:6.1f}%   (Skalierung {scale:.3f})")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print(
        f"\n  Spanne {min(after) * 100:.1f}% bis {max(after) * 100:.1f}% "
        f"(Faktor {max(after) / min(after):.1f}x), Ziel {target * 100:.1f}%"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
