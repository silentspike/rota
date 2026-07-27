#!/usr/bin/env python3
"""Erzeugt die Dateiliste fuer das Symbol-Pruefblatt (docs/icon-check/).

Das Pruefblatt zeigt jedes Symbol aus assets/img/icons/ auf dunklem Grund, auf
hellem Grund und als Rohdatei. Damit die Seite ohne Server-Logik auskommt und
trotzdem nie veraltet, wird die Liste hier erzeugt statt von Hand gepflegt.

Aufruf:

    python3 tools/icon-check-page.py            # icons.js neu schreiben
    python3 tools/icon-check-page.py --check    # nur pruefen, ob sie stimmt
"""

from __future__ import annotations

import argparse
import glob
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS = os.path.join(REPO, "assets", "img", "icons")
PAGE = os.path.join(REPO, "docs", "icon-check")
OUT = os.path.join(PAGE, "icons.js")

HEADER = (
    "// Erzeugt von tools/icon-check-page.py - nicht von Hand aendern.\n"
    "// Die Pfade werden von nginx auf assets/img/icons/ abgebildet (Location\n"
    "// /rota-e9-icons/icons/), damit das Blatt den ausgelieferten Stand zeigt.\n"
)


def build() -> tuple[str, int]:
    """Erzeugt die Liste im Prettier-Stil des Projekts.

    Prettier prueft alles unter docs/ mit (`npm run lint`). Wuerde hier
    JSON-Syntax mit doppelten Anfuehrungszeichen stehen, formatierte Prettier
    die Datei um - und der --check-Lauf meldete sie danach als veraltet.
    Einfache Anfuehrungszeichen und abschliessendes Komma sind deshalb kein
    Geschmack, sondern Bedingung.
    """
    files = sorted(glob.glob(os.path.join(ICONS, "**", "*.svg"), recursive=True))
    rel = ["icons/" + os.path.relpath(f, ICONS).replace(os.sep, "/") for f in files]
    lines = "".join(f"  '{path}',\n" for path in rel)
    return HEADER + "window.__ICONS = [\n" + lines + "];\n", len(files)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv[1:])

    content, count = build()

    if args.check:
        if not os.path.exists(OUT):
            print(f"{os.path.relpath(OUT, REPO)} fehlt")
            return 1
        with open(OUT, encoding="utf-8") as fh:
            current = fh.read()
        if current != content:
            print(f"{os.path.relpath(OUT, REPO)} ist veraltet - neu erzeugen")
            return 1
        print(f"{os.path.relpath(OUT, REPO)}: aktuell, {count} Symbole")
        return 0

    os.makedirs(PAGE, exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as fh:
        fh.write(content)
    print(f"{os.path.relpath(OUT, REPO)}: {count} Symbole geschrieben")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
