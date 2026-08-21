#!/usr/bin/env bash
# Prueft, ob jeder interne Verweis (#anker) in den Dokumenten eine Ueberschrift trifft.
#
# WARUM ES DEN WAECHTER GIBT: Am 2026-08-21 stand in WORKER-GUIDE.md ein Verweis auf
# 8.72, dessen Anker den Titelzusatz nach dem Gedankenstrich nicht enthielt. Der Link
# war tot, seit die Ueberschrift ihren Zusatz bekam — ein Verweis altert mit der
# Ueberschrift, auf die er zeigt, und nichts meldet es.
#
# Er ist ROT eingecheckt worden, im Sinne von 8.28: gegen den Stand von main hat er
# genau diesen einen Verweis gefunden. Ein Waechter, der bei seiner Einfuehrung gruen
# ist, hat nichts bewiesen.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - "$@" <<'PY'
import re, sys, glob, os

def anker(h: str) -> str:
    a = re.sub(r'[^\w\s-]', '', h.strip().lower(), flags=re.UNICODE)
    return re.sub(r'\s+', '-', a.strip())

dateien = sorted(glob.glob('docs/**/*.md', recursive=True)) + ['CLAUDE.md']
fehler = 0
geprueft = 0

for pfad in dateien:
    if not os.path.exists(pfad):
        continue
    s = open(pfad, encoding='utf-8').read()
    vorhanden = {anker(m.group(2)) for m in re.finditer(r'^(#{1,6})\s+(.*)$', s, re.M)}
    # Nur dateiinterne Verweise: ](#ziel) — nicht ](DATEI.md#ziel)
    for zeile_nr, zeile in enumerate(s.split('\n'), 1):
        for ziel in re.findall(r'\]\(#([^)]+)\)', zeile):
            geprueft += 1
            if ziel not in vorhanden:
                print(f'{pfad}:{zeile_nr}: toter Verweis  #{ziel}')
                fehler += 1
    if vorhanden:
        print(f'{pfad}: {len(vorhanden)} Ueberschriften')

# Ein Zaehler, der bei null einen Erfolg meldet, kann nicht von "nichts gefunden"
# unterschieden werden (8.113). Deshalb bricht der Waechter ab, wenn er gar nichts
# zu pruefen hatte — das hiesse, das Suchmuster trifft nicht mehr.
if geprueft == 0:
    print('FEHLER: kein einziger interner Verweis gefunden — das Suchmuster greift nicht mehr.')
    sys.exit(2)

print(f'{geprueft} interne Verweise geprueft, {fehler} tot.')
sys.exit(1 if fehler else 0)
PY
