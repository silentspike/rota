"""Ordnet jede Null im Hebel nach (a), (b) oder (c) ein — Issue #576 AC2/AC4.

    python3 tools/groessen-einordnen.py

Eine 0,00 % im Hebel hat drei Ursachen, die im Ergebnis gleich aussehen:

    (a) der Träger wird in keiner Aufzeichnung gewirkt
    (b) der Träger wirkt, aber die gemessene Größe kann ihn nicht sehen
    (c) der Träger wirkt, die Größe passt — und der Wert ändert nichts

Nur (a) und (c) sind Befunde. (b) ist ein Fehler des Messenden, und genau der
war die erste Fassung: Sie maß `expected.damage`, und `salve` heilt.

# Warum (b) hier nicht mehr auftreten KANN

Das Hebel-Werkzeug vergleicht seit #576 alle fünf Größen. Eine Null bedeutet
damit: keine der fünf bewegt sich. Ob der Träger gewirkt wird, entscheidet
zwischen (a) und (c) — und beides steht in den Aufzeichnungen.

Das ist die eigentliche Leistung: Nicht die Nullen zu beseitigen, sondern (b)
strukturell auszuschließen, damit eine Null wieder etwas bedeutet.
"""

import json
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
AUFZ = WURZEL / 'tests' / 'parity' / 'recordings'


def gewirkte_zauber():
    """Welche Zauber in irgendeiner Aufzeichnung GEDRÜCKT werden."""
    gewirkt = set()
    for pfad in AUFZ.glob('*.json'):
        roh = json.loads(pfad.read_text(encoding='utf8'))
        for lauf in roh['runs'] if 'runs' in roh else [roh]:
            # **Zwei Press-Formen im Bestand**, gemessen: {'slot','tick'} und
            # {'spellId','tick'}. Ein Werkzeug, das nur die erste kennt,
            # bricht mit KeyError — was hier besser ist als die Alternative,
            # denn ein `.get('slot')` hätte still `None` geliefert und den
            # Zauber als „nie gewirkt" eingestuft.
            drucke = lauf.get('presses', [])
            gewirkt |= {p['spellId'] for p in drucke if 'spellId' in p}
            slots = {p['slot'] for p in drucke if 'slot' in p}
            for bindung in lauf.get('bindings', []) or roh.get('bindings', []):
                if bindung['slot'] in slots:
                    gewirkt.add(bindung['spellId'])
    return gewirkt


def main():
    gewirkt = gewirkte_zauber()
    zeilen = (WURZEL / 'docs/groessen-hebel-messung.txt').read_text().splitlines()
    zaehler = {'Hebel': 0, '(a) nie gewirkt': 0, '(c) wirkt ohne Wirkung': 0}

    for zeile in zeilen:
        teile = zeile.split()
        if len(teile) < 4:
            continue
        hebel, feld, zauber = teile[0], teile[1], teile[2]
        if hebel != '0.00%':
            zaehler['Hebel'] += 1
            continue
        art = '(c) wirkt ohne Wirkung' if zauber in gewirkt else '(a) nie gewirkt'
        zaehler[art] += 1
        print(f'{art:24} {feld:26} {zauber}')

    print()
    for art, n in zaehler.items():
        print(f'{art:24} {n}')
    print(f'\n(b) falsche Messgröße    0  — strukturell ausgeschlossen, siehe Dateikopf')


if __name__ == '__main__':
    main()
