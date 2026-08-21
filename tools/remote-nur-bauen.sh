#!/usr/bin/env bash
# Prueft, dass niemand einen LAUF auf den Bauserver schickt.
#
# DIE REGEL steht in /work/.claude/CLAUDE.md unter "## Rust — Remote-Build":
#
#     cargo remote -c -- test --no-run   # NUR bauen, nie ausfuehren
#
# Begruendung dort: rustbuild hat kein AVX2/FMA. Die Zielmaschine ist der Laptop
# (210 094 gegen 121 689 Kaempfe/s) — ein Lauf auf dem Bauserver misst eine
# ANDERE Maschine als die, fuer die gebaut wird. Erlaubt sind `build`,
# `test --no-run`, `clippy` und `cargo-remote-fmt`.
#
# WARUM ES DEN WAECHTER GIBT: Am 2026-08-21 standen 28 Stellen im Repository,
# die genau das Gegenteil empfahlen — 25 aelter als die Klarstellung vom
# 18.08., zwei einen Tag juenger, und eine in AUSFUEHRENDEM Code.
#
# Er ist ROT eingecheckt worden (8.28): Gegen den Stand von main findet er diese
# 28. Ein Waechter, der bei seiner Einfuehrung gruen ist, hat nichts bewiesen.
#
# WARUM `git ls-files` UND KEIN `--include`: Der Befund selbst entstand aus einem
# zu engen Suchraum. Ein Endungsfilter sieht vollstaendig aus, weil man ihn
# selbst geschrieben hat — meiner kannte rs, md, sh und toml, und die schwerste
# Stelle stand in einer `.py`. Was versioniert ist, wird durchsucht; ueber die
# Endung entscheidet niemand mehr.
set -uo pipefail
cd "$(dirname "$0")/.."

python3 - "$@" <<'PY'
import re, subprocess, sys

# `-- run`, `-- bench` und `-- test` OHNE --no-run schicken einen Lauf hinueber.
AUSFUEHREND = re.compile(r"cargo\s+remote\b[^\n]*--\s+(run|bench|test)\b")
ERLAUBT     = re.compile(r"--no-run")

# ZWEI KONTROLLDATEIEN, deren Erreichbarkeit vorher feststeht.
#
# GEPRUEFT WIRD, OB DER SUCHRAUM SIE LIEST — nicht, ob sie einen Verstoss
# enthalten. Der Unterschied ist der Grund, warum die erste Fassung dieses
# Waechters in dem Moment ansprang, in dem die Verstoesse behoben waren: Sie
# suchte die Kontrollen unter den TREFFERN, und ein behobener Treffer ist keiner
# mehr. Eine Kontrolle, die verschwindet, sobald die Arbeit getan ist, misst die
# Arbeit statt den Suchraum.
#
# `.cargo-remote.toml` liegt in der Wurzel, `tools/streich-mutation.py` ist eine
# `.py` — an dieser Endung ist der Filter gescheitert, aus dem dieser Befund
# entstand.
KONTROLLE = [".cargo-remote.toml", "tools/streich-mutation.py"]

# BENANNTE AUSNAHME, mit Grund und Frist — nicht aus dem Suchraum genommen.
#
# Der Waechter FINDET diese Zeile weiter und MELDET sie. Sie zaehlt nur nicht
# als Verstoss, solange #1045 offen ist: Sie steht in ausfuehrendem Code, und
# ihre Umstellung ist eine Verhaltensaenderung an einem Werkzeug, das stundenlang
# laeuft — kein Textkorrektur-Nebenzug.
#
# UND SIE WIRD ROT, SOBALD SIE UEBERHOLT IST: Verschwindet die Zeile (weil #1045
# sie umgestellt hat), meldet der Waechter die Ausnahme als tot. Eine stumme
# Ausnahme waere das Gegenteil dessen, wofuer er da ist.
#
# Erkannt am INHALT, nicht an der Zeilennummer — die wandert bei jeder Aenderung
# darueber.
# Leer, und das ist der Sollzustand. Der eine Eintrag, der hier stand, betraf
# `tools/streich-mutation.py` — er war ausdruecklich befristet auf die Dauer von
# #1045, und mit dem Umbau des Skripts auf "remote bauen, lokal fahren" gibt es
# die Zeile nicht mehr.
#
# Ein Eintrag hier ist keine Erlaubnis auf Dauer, sondern eine Schuld: Er nennt
# das Issue, das ihn wieder entfernt. Steht ein Grund ohne Issue-Nummer da, ist
# die Frist unbestimmt — dann fehlt der Termin, an dem jemand nachsieht.
AUSNAHMEN = {}

dateien = subprocess.run(["git", "ls-files"], capture_output=True, text=True).stdout.split("\n")
treffer = []
# GELESEN, nicht nur AUFGELISTET. Ein Pfad in `git ls-files` sagt nur, dass die
# Datei versioniert ist — nicht, dass das `open()` gelungen ist. Ein Lesefehler
# wuerde sie sonst als erreicht zaehlen, obwohl der Waechter nie hineingesehen hat.
durchsucht = set()
for pfad in dateien:
    if not pfad: continue
    try:
        with open(pfad, encoding="utf-8", errors="ignore") as fh:
            for nr, zeile in enumerate(fh, 1):
                if AUSFUEHREND.search(zeile) and not ERLAUBT.search(zeile):
                    treffer.append((pfad, nr, zeile.strip()))
        durchsucht.add(pfad)
    except (IsADirectoryError, PermissionError):
        continue

verstoss, geduldet = [], []
for pfad, nr, zeile in treffer:
    grund = next((g for (d, m), g in AUSNAHMEN.items() if pfad == d and m in zeile), None)
    (geduldet if grund else verstoss).append((pfad, nr, zeile, grund))

for pfad, nr, zeile, _ in verstoss:
    print(f"{pfad}:{nr}: {zeile[:96]}")
for pfad, nr, zeile, grund in geduldet:
    print(f"GEDULDET  {pfad}:{nr}: {zeile[:60]}")
    print(f"          {grund}")

print()
print(f"{len(durchsucht)} versionierte Dateien gelesen, "
      f"{len(verstoss)} Laeufe gefunden, {len(geduldet)} geduldet.")

# Die Kontrolldateien: erreicht der Suchraum sie ueberhaupt?
fehlend = [k for k in KONTROLLE if k not in durchsucht]
if fehlend:
    print()
    print("SUCHRAUM ZU ENG — diese Kontrolldateien wurden nicht gelesen:")
    for k in fehlend:
        print(f"  {k}")
    print("Solange sie fehlen, sagt eine niedrige Trefferzahl nichts.")
    sys.exit(2)

# ERST HIER, nach der Suchraum-Pruefung: Eine Ausnahme, deren Zeile es nicht mehr
# gibt, ist ueberholt. Davor waere dieselbe Meldung die falsche Diagnose — ein zu
# enger Suchraum laesst die Zeile ebenfalls verschwinden.
tot = [f"{d} ({m})" for (d, m) in AUSNAHMEN
       if not any(p == d and m in z for p, _, z, _ in geduldet)]
if tot:
    print()
    print("AUSNAHME UEBERHOLT — die Zeile gibt es nicht mehr, der Eintrag gehoert raus:")
    for t in tot:
        print(f"  {t}")
    sys.exit(3)

sys.exit(1 if verstoss else 0)
PY
