#!/usr/bin/env python3
"""Prueft, ob zwei Zauber-Symbole in Spielgroesse verwechselbar sind (Issue #21).

WARUM DAS NOETIG IST
--------------------
Der Spieler entscheidet im Millisekundenbereich und schaut das Symbol dabei
nicht an - er sieht es am Rand des Blickfelds. Zwei Symbole, die sich unter
diesen Bedingungen aehneln, machen aus der Rotation ein Ratespiel
(GAME-DESIGN 17.2).

WIE GEMESSEN WIRD
-----------------
Nicht die Datei wird verglichen, sondern das, was das Auge unter Zeitdruck
davon aufnimmt:

  1. Rastern auf Spielgroesse (72 px; E1 zeichnet mit SLOT_SIZE = 92)
  2. Verkleinern auf 32 x 32 und weichzeichnen - der kurze, periphere Blick
     loest keine feinen Strukturen auf
  3. Nur der Alphakanal zaehlt. Die Farbe kommt im Spiel aus der Schule und
     kann zwei Symbole nicht auseinanderhalten, wenn beide zur selben Schule
     gehoeren

Hauptmass ist die FORMKORRELATION (Pearson) ueber die weichgezeichnete Maske:
1,0 = deckungsgleich, 0 = kein Zusammenhang. Daneben laufen zwei
Diagnose-Masse mit, die erklaeren, WARUM ein Paar auffaellt:

  Ueberlappung  Intersection over Union der harten Silhouette
  Grobstruktur  Korrelation ueber ein 4 x 4-Raster der Deckungsanteile -
                "wo sitzt die Masse", ganz ohne Detail

DER SCHWELLWERT
---------------
Er ist nicht geraten, sondern kalibriert (--calibrate). Das Fenster wird von
zwei Seiten eingegrenzt, jeweils ueber ALLE Symbole gemessen:

  OBERGRENZE - was auf jeden Fall anschlagen muss
    dasselbe Symbol um 8 % skaliert      schwaechster Fall 0,913
    dasselbe Symbol um 3 px versetzt     schwaechster Fall 0,854
  Beides waere im Spiel dasselbe Bild. Der Schwellwert muss darunter liegen.

  UNTERGRENZE - wo das Grundrauschen des Satzes endet
    99-%-Quantil aller echten Paare      0,824
    Median + 3 x MAD                     0,844
  Darunter lebt der normale Abstand zwischen verschiedenen Symbolen.

Gespiegelte Symbole sind BEWUSST kein Muss-Kriterium: Bei einer asymmetrischen
Form faellt die Korrelation auf 0,43, und im Spiel ist ein gespiegeltes Motiv
tatsaechlich unterscheidbar. Der Wert laeuft als Diagnose mit.

Aufruf:

    python3 tools/icon-distinct.py                 # alle Paare pruefen
    python3 tools/icon-distinct.py --top 15        # nur die aehnlichsten
    python3 tools/icon-distinct.py --calibrate     # Schwellwert begruenden
    python3 tools/icon-distinct.py --selftest      # Gegenprobe (AC3)
"""

from __future__ import annotations

import argparse
import glob
import itertools
import os
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS = os.path.join(REPO, "assets", "img", "icons")

# Die Zauberleiste zeichnet mit SLOT_SIZE = 92 (src/render/hud.ts); das Symbol
# sitzt mit Rand darin.
GAME_SIZE = 72

# Aufloesung des Vergleichs. Feiner waere nicht besser: Der kurze Blick loest
# Details ohnehin nicht auf, und feine Unterschiede wuerden echte Aehnlichkeit
# ueberdecken.
COMPARE_SIZE = 32

# Weichzeichnung in Pixeln der Vergleichsaufloesung.
BLUR = 1.1

# Kalibriert - siehe --calibrate und der Kopf dieser Datei. Das belegte Fenster
# ist 0,824 bis 0,854; 0,84 sitzt darin und faellt zugleich mit Median + 3 x MAD
# des Satzes zusammen.
THRESHOLD = 0.84


def raster_alpha(path: str, size: int):
    """Symbol in Spielgroesse rastern und den Alphakanal zurueckgeben."""
    import numpy as np
    from PIL import Image

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        png = tmp.name
    try:
        subprocess.run(
            ["inkscape", "-w", str(size), "-h", str(size), path, "-o", png],
            check=True,
            capture_output=True,
        )
        img = Image.open(png).convert("RGBA")
        return np.asarray(img.getchannel("A"), dtype=float) / 255.0
    finally:
        os.unlink(png)


def perceive(alpha):
    """Aus der scharfen Maske das machen, was der kurze Blick davon uebrig laesst."""
    import numpy as np
    from PIL import Image
    from scipy.ndimage import gaussian_filter

    small = np.asarray(
        Image.fromarray((alpha * 255).astype("uint8")).resize(
            (COMPARE_SIZE, COMPARE_SIZE), Image.LANCZOS
        ),
        dtype=float,
    )
    return gaussian_filter(small / 255.0, sigma=BLUR)


def correlation(a, b) -> float:
    """Pearson-Korrelation zweier Bilder. 1,0 = deckungsgleich."""
    import numpy as np

    x = a.ravel() - a.mean()
    y = b.ravel() - b.mean()
    nx, ny = np.linalg.norm(x), np.linalg.norm(y)
    if nx == 0 or ny == 0:
        return 0.0
    return float(np.dot(x, y) / (nx * ny))


def iou(a, b) -> float:
    """Ueberlappung der harten Silhouette."""
    import numpy as np

    pa, pb = a > 0.5, b > 0.5
    union = np.logical_or(pa, pb).sum()
    return float(np.logical_and(pa, pb).sum() / union) if union else 0.0


def coarse(alpha):
    """4 x 4-Raster der Deckungsanteile - wo sitzt die Masse."""
    import numpy as np

    n = alpha.shape[0] // 4
    return np.array([[alpha[r * n : (r + 1) * n, c * n : (c + 1) * n].mean() for c in range(4)] for r in range(4)])


class Icons:
    """Laedt die Symbole einmal und haelt die vorverarbeiteten Fassungen."""

    def __init__(self, folder: str, size: int):
        self.paths = sorted(glob.glob(os.path.join(folder, "**", "*.svg"), recursive=True))
        self.names = [os.path.relpath(p, folder) for p in self.paths]
        self.alpha = [raster_alpha(p, size) for p in self.paths]
        self.view = [perceive(a) for a in self.alpha]
        self.grid = [coarse(v) for v in self.view]

    def __len__(self) -> int:
        return len(self.paths)

    def pair(self, i: int, j: int) -> tuple[float, float, float]:
        return (
            correlation(self.view[i], self.view[j]),
            iou(self.view[i], self.view[j]),
            correlation(self.grid[i], self.grid[j]),
        )


def all_pairs(icons: Icons) -> list[tuple[float, float, float, str, str]]:
    rows = []
    for i, j in itertools.combinations(range(len(icons)), 2):
        form, over, grob = icons.pair(i, j)
        rows.append((form, over, grob, icons.names[i], icons.names[j]))
    rows.sort(key=lambda r: -r[0])
    return rows


def cmd_check(icons: Icons, top: int | None, threshold: float) -> int:
    rows = all_pairs(icons)
    shown = rows if top is None else rows[:top]

    print(f"{len(icons)} Symbole, {len(rows)} Paare, Schwellwert {threshold:.2f}\n")
    print(f"{'Form':>6}{'Ueberl.':>9}{'Grob':>7}   Paar")
    print("-" * 78)
    for form, over, grob, a, b in shown:
        mark = "  <-- ZU AEHNLICH" if form >= threshold else ""
        print(f"{form:6.3f}{over:9.3f}{grob:7.3f}   {a} / {b}{mark}")
    if top is not None and len(rows) > top:
        print(f"... {len(rows) - top} weitere Paare unter {shown[-1][0]:.3f}")
    print("-" * 78)

    over_threshold = [r for r in rows if r[0] >= threshold]
    if over_threshold:
        print(f"NICHT BESTANDEN: {len(over_threshold)} Paar(e) ueber dem Schwellwert")
        for form, _o, _g, a, b in over_threshold:
            print(f"  {form:.3f}  {a} / {b}")
        return 1
    print(f"BESTANDEN: kein Paar erreicht {threshold:.2f}")
    print(f"  hoechster Wert: {rows[0][0]:.3f}  ({rows[0][3]} / {rows[0][4]})")
    print(f"  Median:         {rows[len(rows) // 2][0]:.3f}")
    return 0


def variants(alpha):
    """Fassungen desselben Symbols, die im Spiel dasselbe Bild waeren.

    Nur die Skalierung zaehlt als Muss-Fall. Ein VERSATZ gehoert bewusst nicht
    dazu: Bei einer duennen, gestreckten Form - etwa der Schraubzwinge - faellt
    die Korrelation schon bei 3 px auf 0,76, und dort ist der Versatz auch
    tatsaechlich zu sehen. Ihn trotzdem als "dasselbe Bild" zu fuehren, wuerde
    die Obergrenze des Fensters an das duennste Symbol des Satzes koppeln
    statt an die Wahrnehmung. Versatz und Spiegelung laufen als Diagnose mit.
    """
    from PIL import Image
    import numpy as np

    img = Image.fromarray((alpha * 255).astype("uint8"))
    small = img.resize((int(img.width * 0.92), int(img.height * 0.92)), Image.LANCZOS)
    canvas = Image.new("L", img.size, 0)
    canvas.paste(small, ((img.width - small.width) // 2, (img.height - small.height) // 2))
    return {"um 8 % skaliert": np.asarray(canvas, dtype=float) / 255.0}


def diagnostics(alpha):
    """Motivabhaengige Faelle - aufschlussreich, aber kein Kriterium."""
    import numpy as np

    return {"um 3 px versetzt": np.roll(alpha, 3, axis=1), "gespiegelt": np.fliplr(alpha)}


def cmd_calibrate(icons: Icons, threshold: float) -> int:
    """Belegt, dass der Schwellwert an der richtigen Stelle liegt.

    Gemessen ueber ALLE Symbole, nicht an einem Beispiel: Der schwaechste
    Muss-Fall bestimmt die Obergrenze, das Grundrauschen die Untergrenze.
    """
    import numpy as np

    must_hit: dict[str, list[float]] = {}
    diag: dict[str, list[float]] = {}
    for alpha, view in zip(icons.alpha, icons.view):
        for label, variant in variants(alpha).items():
            must_hit.setdefault(label, []).append(correlation(view, perceive(variant)))
        for label, variant in diagnostics(alpha).items():
            diag.setdefault(label, []).append(correlation(view, perceive(variant)))

    rows = all_pairs(icons)
    vals = np.array([r[0] for r in rows])
    med = float(np.median(vals))
    mad = float(np.median(np.abs(vals - med)))
    q99 = float(np.quantile(vals, 0.99))

    print(f"KALIBRIERUNG - {len(icons)} Symbole, {len(rows)} echte Paare\n")
    print("MUSS ANSCHLAGEN - waere im Spiel dasselbe Bild")
    print(f"{'Fall':<34}{'schwaechster':>14}{'Median':>9}")
    print("-" * 57)
    for label, values in must_hit.items():
        print(f"{label:<34}{min(values):14.3f}{float(np.median(values)):9.3f}")
    ceiling = min(min(v) for v in must_hit.values())

    print("\nGRUNDRAUSCHEN - normaler Abstand zwischen verschiedenen Symbolen")
    print(f"{'Median aller Paare':<34}{med:14.3f}")
    print(f"{'Median + 3 x MAD':<34}{med + 3 * mad:14.3f}")
    print(f"{'99-%-Quantil':<34}{q99:14.3f}")
    # Bewusst der KLEINERE der beiden Ausreisser-Schaetzer: Er macht die
    # Pruefung strenger. Ein Fehlalarm kostet einen Symboltausch, ein
    # uebersehenes Paar kostet im Spiel eine Fehlentscheidung pro Kampf.
    floor = min(q99, med + 3 * mad)

    print("\nDIAGNOSE - motivabhaengig, deshalb kein Kriterium")
    print(f"{'Fall':<34}{'schwaechster':>14}{'Median':>9}")
    for label, values in diag.items():
        print(f"{label:<34}{min(values):14.3f}{float(np.median(values)):9.3f}")
    print("  Duenne Formen verlieren bei Versatz und Spiegelung stark - dort ist")
    print("  der Unterschied im Spiel auch tatsaechlich zu sehen.")

    print(f"\nFenster: {floor:.3f} bis {ceiling:.3f}")
    print(f"gesetzt: {threshold:.2f}")
    if not (floor <= threshold <= ceiling):
        print("\nNICHT BESTANDEN: Der Schwellwert liegt ausserhalb des belegten Fensters.")
        return 1
    print(f"Abstand nach unten {threshold - floor:.3f}, nach oben {ceiling - threshold:.3f}")
    print("BESTANDEN")
    return 0


def cmd_worst(icons: Icons, threshold: float) -> int:
    """Zeigt je Symbol, wie eng es am naechsten Nachbarn sitzt.

    Ein einzelnes zu aehnliches Paar taeuscht leicht darueber hinweg, dass die
    Enge von wenigen Symbolen getragen wird: Wer in mehreren Spitzenpaaren
    steckt, ist der Kandidat zum Austausch - nicht sein jeweiliger Partner.
    """
    rows = all_pairs(icons)
    nearest: dict[str, tuple[float, str]] = {}
    count: dict[str, int] = {name: 0 for name in icons.names}
    for form, _o, _g, a, b in rows:
        for me, other in ((a, b), (b, a)):
            if me not in nearest or form > nearest[me][0]:
                nearest[me] = (form, other)
        if form >= threshold - 0.06:
            count[a] += 1
            count[b] += 1

    order = sorted(icons.names, key=lambda n: -nearest[n][0])
    print(f"{len(icons)} Symbole, Schwellwert {threshold:.2f}\n")
    print(f"{'naechster':>10}{'in Spitze':>11}   Symbol / Nachbar")
    print("-" * 78)
    for name in order:
        form, other = nearest[name]
        mark = "  <--" if form >= threshold else ""
        print(f"{form:10.3f}{count[name]:11d}   {name} / {other}{mark}")
    print("-" * 78)
    print("Spalte 'in Spitze': in wie vielen Paaren nahe am Schwellwert das Symbol steckt.")
    return 0


def cmd_suggest(icons: Icons, pool: str, replace: str, threshold: float, size: int) -> int:
    """Sucht Ersatz fuer ein zu aehnliches Symbol.

    "Zu aehnliche Paare neu erzeugen" heisst bei GEAETZT nicht erzeugen, sondern
    AUSTAUSCHEN: game-icons.net haelt 4.181 Symbole bereit. Der Kandidat wird
    dafuer genauso normalisiert wie der Bestand (tools/icon-normalize.py), sonst
    vergleicht man Aepfel mit Birnen.

    Den Pool bekommt man ueber
        https://game-icons.net/archives/ffffff/transparent/game-icons.net.svg.zip
    und entpackt ihn irgendwohin ausserhalb des Repositorys.

    Welcher Kandidat inhaltlich zur Schule passt, entscheidet ein Mensch. Dieses
    Werkzeug sagt nur, welche ueberhaupt in Frage kommen.
    """
    import shutil

    if replace not in icons.names:
        print(f"{replace} ist nicht im Bestand. Vorhanden: {', '.join(icons.names[:4])} ...")
        return 1

    keep = [i for i, n in enumerate(icons.names) if n != replace]
    print(f"Ersatz fuer {replace}, geprueft gegen {len(keep)} verbleibende Symbole")
    print(f"Schwellwert {threshold:.2f}\n")

    candidates = sorted(glob.glob(os.path.join(pool, "**", "*.svg"), recursive=True))
    if not candidates:
        print(f"Keine Kandidaten unter {pool}")
        return 1
    print(f"{len(candidates)} Kandidaten\n")

    normalize = os.path.join(REPO, "tools", "icon-normalize.py")
    results = []
    with tempfile.TemporaryDirectory() as tmpdir:
        for cand in candidates:
            work = os.path.join(tmpdir, "one")
            os.makedirs(work, exist_ok=True)
            staged = os.path.join(work, os.path.basename(cand))
            shutil.copyfile(cand, staged)
            # Gleiche Vorbehandlung wie der Bestand - sonst ist der Vergleich wertlos.
            subprocess.run(
                [sys.executable, normalize, "--icons", work],
                check=True,
                capture_output=True,
            )
            view = perceive(raster_alpha(staged, size))
            worst = max((correlation(view, icons.view[i]), icons.names[i]) for i in keep)
            results.append((worst[0], worst[1], os.path.relpath(cand, pool)))
            shutil.rmtree(work)

    results.sort()
    print(f"{'schlecht.':>10}  {'naechster Nachbar':<34}Kandidat")
    print("-" * 78)
    for worst, neighbour, name in results:
        mark = "" if worst < threshold else "  <-- faellt durch"
        print(f"{worst:10.3f}  {neighbour:<34}{name}{mark}")
    print("-" * 78)
    usable = [r for r in results if r[0] < threshold]
    print(f"{len(usable)} von {len(results)} Kandidaten bleiben unter {threshold:.2f}")
    return 0 if usable else 1


def cmd_selftest(icons: Icons, threshold: float) -> int:
    """Gegenprobe (AC3): Ein absichtlich doppeltes Symbol MUSS auffallen."""
    import shutil

    with tempfile.TemporaryDirectory() as tmpdir:
        stage = os.path.join(tmpdir, "icons")
        shutil.copytree(ICONS, stage)
        original = icons.paths[0]
        rel = icons.names[0]
        clone = os.path.join(stage, "zzz-doppelt.svg")
        shutil.copyfile(original, clone)
        print(f"Gegenprobe: {rel} zusaetzlich als zzz-doppelt.svg eingeschleust\n")

        salted = Icons(stage, GAME_SIZE)
        rows = all_pairs(salted)
        hits = [r for r in rows if r[0] >= threshold]

        print(f"{len(salted)} Symbole, {len(rows)} Paare, Schwellwert {threshold:.2f}")
        for form, over, grob, a, b in rows[:3]:
            print(f"  {form:6.3f}{over:9.3f}{grob:7.3f}   {a} / {b}")

        found = [r for r in hits if "zzz-doppelt.svg" in (r[3], r[4])]
        if not found:
            print("\nNICHT BESTANDEN: Das Duplikat wurde nicht erkannt.")
            return 1
        print(f"\nBESTANDEN: Duplikat erkannt mit Form {found[0][0]:.3f}")
        fremd = [r for r in hits if "zzz-doppelt.svg" not in (r[3], r[4])]
        if fremd:
            print(f"WARNUNG: {len(fremd)} weitere Paare ueber dem Schwellwert")
            return 1
        print("Keine Fehlalarme: nur das Duplikat schlaegt an.")
        return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--icons", default=ICONS)
    parser.add_argument("--size", type=int, default=GAME_SIZE)
    parser.add_argument("--threshold", type=float, default=THRESHOLD)
    parser.add_argument("--top", type=int, default=None, help="nur die N aehnlichsten Paare zeigen")
    parser.add_argument("--calibrate", action="store_true", help="Schwellwert belegen")
    parser.add_argument("--selftest", action="store_true", help="Gegenprobe mit Duplikat")
    parser.add_argument("--worst", action="store_true", help="je Symbol den naechsten Nachbarn zeigen")
    parser.add_argument("--suggest", metavar="POOL", help="Ersatz aus einem Symbol-Vorrat suchen")
    parser.add_argument("--replace", metavar="NAME", help="welches Symbol ersetzt werden soll")
    args = parser.parse_args(argv[1:])

    icons = Icons(args.icons, args.size)
    if len(icons) < 2:
        print(f"Zu wenige Symbole unter {args.icons}")
        return 1

    if args.calibrate:
        return cmd_calibrate(icons, args.threshold)
    if args.selftest:
        return cmd_selftest(icons, args.threshold)
    if args.worst:
        return cmd_worst(icons, args.threshold)
    if args.suggest:
        if not args.replace:
            print("--suggest braucht --replace <name>")
            return 1
        return cmd_suggest(icons, args.suggest, args.replace, args.threshold, args.size)
    return cmd_check(icons, args.top, args.threshold)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
