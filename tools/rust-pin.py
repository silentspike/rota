#!/usr/bin/env python3
"""Ein Rust-Pin für CI und Bau-Dienst — gelesen, nicht abgeschrieben.

Zwei Dateien legen den Compiler fest, und beide werden gebraucht:

    rust-toolchain.toml   `channel`    damit baut die CI (rustup-Override)
    .cargo-remote.toml    `toolchain`  damit baut cargo remote v2

Weichen sie ab, prüft die CI einen anderen Compiler als den, mit dem der
Bau-Dienst gebaut hat — und beide melden grün. Der v2-Client nimmt seinen Wert
aus `.cargo-remote.toml` und übergeht `rust-toolchain.toml`; eine Änderung an
nur einer Datei fällt deshalb nirgends von selbst auf.

    python3 -B tools/rust-pin.py                              # prüft, druckt die Version
    python3 -B tools/rust-pin.py --rustc "$(rustc --version)" # und: läuft genau sie?

Auf stdout steht NUR die Version — die CI reicht sie an dtolnay/rust-toolchain
weiter. Befunde gehen nach stderr. Exit 0 einheitlich, 1 bei jedem Befund.
"""

import argparse
import re
import sys
import tomllib
from pathlib import Path

EXAKT = re.compile(r"\d+\.\d+\.\d+")
# Die Kopfzeile des verwalteten Abschnitts. Der v2-Client trennt die Datei an
# ihrem ERSTEN Vorkommen; steht sie zusätzlich in einem Kommentar, schreibt er
# den Abschnitt nach dem nächsten Lauf doppelt, und die Datei ist kein gültiges
# TOML mehr (gemessen am 2026-10-23, Kommentar in .cargo-remote.toml).
KOPF = "[cargo_remote]"


def befunde(wurzel, rustc=None):
    """Liefert (Version oder None, Liste der Befunde)."""
    fehler = []
    kanal = None
    try:
        kette = tomllib.loads((wurzel / "rust-toolchain.toml").read_text(encoding="utf-8"))
        kanal = kette.get("toolchain", {}).get("channel")
    except FileNotFoundError:
        fehler.append("rust-toolchain.toml fehlt — die CI liefe auf ihrer Voreinstellung")
    except tomllib.TOMLDecodeError as exc:
        fehler.append(f"rust-toolchain.toml ist kein gültiges TOML: {exc}")
    if kanal is not None and not (isinstance(kanal, str) and EXAKT.fullmatch(kanal)):
        fehler.append(f"rust-toolchain.toml: channel = {kanal!r} ist keine exakte Version x.y.z")
        kanal = None
    elif kanal is None and not fehler:
        fehler.append("rust-toolchain.toml: kein channel unter [toolchain]")

    pfad = wurzel / ".cargo-remote.toml"
    try:
        roh = pfad.read_text(encoding="utf-8")
    except FileNotFoundError:
        roh = None
        fehler.append(".cargo-remote.toml fehlt — der Bau-Dienst kennt das Projekt nicht")
    if roh is not None:
        anzahl = roh.count(KOPF)
        if anzahl != 1:
            fehler.append(f".cargo-remote.toml: die Kopfzeile {KOPF} steht {anzahl}-mal da, erlaubt ist "
                          "genau einmal — der Client zerlegt die Datei sonst beim nächsten Lauf")
        try:
            dienst = tomllib.loads(roh).get("cargo_remote")
        except tomllib.TOMLDecodeError as exc:
            dienst = None
            fehler.append(f".cargo-remote.toml ist kein gültiges TOML: {exc}")
        if dienst is None:
            if anzahl == 1:
                fehler.append(".cargo-remote.toml: der v2-Abschnitt fehlt")
        elif dienst.get("schema") != 1:
            fehler.append(f".cargo-remote.toml: schema = {dienst.get('schema')!r}, erwartet 1")
        elif kanal is not None and dienst.get("toolchain") != kanal:
            fehler.append(f"Abweichung: rust-toolchain.toml sagt {kanal}, .cargo-remote.toml sagt "
                          f"{dienst.get('toolchain')!r} — CI und Bau-Dienst bauten verschieden")

    if rustc is not None and kanal is not None:
        # Die Version muss als GANZES Wort stehen: "rustc 1.98.10 (…)" ist
        # nicht 1.98.1, auch wenn die Zeichenkette damit beginnt.
        gefunden = re.match(r"rustc (\d+\.\d+\.\d+)(?:\s|$)", rustc.strip())
        if not gefunden:
            fehler.append(f"rustc meldet keine lesbare Version: {rustc.strip()!r}")
        elif gefunden.group(1) != kanal:
            fehler.append(f"Es läuft rustc {gefunden.group(1)}, gepinnt ist {kanal}")
    return (kanal if not fehler else None), fehler


def main(argumente=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--rustc", metavar="AUSGABE", help="die Ausgabe von `rustc --version`")
    args = parser.parse_args(argumente)
    version, fehler = befunde(args.root, args.rustc)
    for zeile in fehler:
        print(f"rust-pin: {zeile}", file=sys.stderr)
    if fehler:
        return 1
    print(version)
    return 0


if __name__ == "__main__":
    sys.exit(main())
