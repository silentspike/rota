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

# ZWEI KONTROLLZEILEN, deren Antwort vorher feststeht. Findet der Waechter sie
# nicht, ist sein Suchraum zu eng — und das faellt JETZT auf statt beim naechsten
# Mal. Die zweite ist absichtlich eine `.py`: an ihr ist der Endungsfilter
# gescheitert, der diesen Befund fast halbiert haette.
KONTROLLE = [".cargo-remote.toml", "tools/streich-mutation.py"]

dateien = subprocess.run(["git", "ls-files"], capture_output=True, text=True).stdout.split("\n")
treffer = []
for pfad in dateien:
    if not pfad: continue
    try:
        with open(pfad, encoding="utf-8", errors="ignore") as fh:
            for nr, zeile in enumerate(fh, 1):
                if AUSFUEHREND.search(zeile) and not ERLAUBT.search(zeile):
                    treffer.append((pfad, nr, zeile.strip()))
    except (IsADirectoryError, PermissionError):
        continue

for pfad, nr, zeile in treffer:
    print(f"{pfad}:{nr}: {zeile[:96]}")

print()
print(f"{len(dateien)-1} versionierte Dateien durchsucht, {len(treffer)} Laeufe gefunden.")

# Die Kontrollzeilen: erreicht der Suchraum sie ueberhaupt?
fehlend = [k for k in KONTROLLE if not any(p.endswith(k) for p, _, _ in treffer)]
if fehlend:
    print()
    print("SUCHRAUM ZU ENG — diese Kontrollzeilen wurden nicht erreicht:")
    for k in fehlend:
        print(f"  {k}")
    print("Solange sie fehlen, sagt eine niedrige Trefferzahl nichts.")
    sys.exit(2)

sys.exit(1 if treffer else 0)
PY
