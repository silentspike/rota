"""Fügt für die Gegenprobe (Issue #402) einen Zauber ein — vorne oder hinten.

Steht als eigene Datei neben dem Skript und nicht als Here-Dokument darin: Ein
eingebettetes Python-Fragment endet auf `PY`, und genau dieser Terminator
kollidiert mit dem Here-Dokument der Shell, die es einbettet.

Aufruf ausschließlich über `tools/reihenfolge-gegenprobe.sh` — nur dort werden
die Dateien vorher gesichert und danach zurückgestellt.
"""

import json
import sys
from pathlib import Path


def letzte_quelle():
    """Die Quelle der letzten akzeptierten Kennung, nicht die einst letzte Datei.

    test-cast.json war nur bis zum Hinzufügen der weiteren Prüfquellen die
    letzte Quelle. Die Gegenprobe soll hinter der akzeptierten Reihenfolge
    anfügen; deren letzte Kennung ist ihr vorhandener, unabhängiger Maßstab.
    """
    kennung = json.loads(Path('tests/unit/spell-order.json').read_text())[-1]
    quellen = [p for p in sorted(Path('data/spells').glob('*.json'))
               if any(s['id'] == kennung for s in json.loads(p.read_text())['spells'])]
    if len(quellen) != 1:
        raise RuntimeError(f'Letzte Kennung {kennung}: {len(quellen)} Quellen statt einer')
    return str(quellen[0])


def main():
    modus = sys.argv[1]
    if modus == '--letzte-datei':
        print(letzte_quelle())
        return
    if modus not in {'vorne', 'hinten', 'eigene-datei'}:
        raise SystemExit(f'Unbekannter Einfügemodus: {modus}')
    einfuegen(modus)

# Drei Fälle, und der dritte ist der lehrreiche (Issue #408):
#
#   vorne         an den Anfang der ERSTEN Quelle      -> meldet sich, immer falsch
#   hinten        an das Ende der LETZTEN Quelle       -> geht ohne Nachziehen durch
#   eigene-datei  an das Ende von calcination.json     -> meldet sich, ist aber
#                                                         seit 2026-08-06 der
#                                                         NORMALE Fall
#
# Der dritte sieht aus wie der zweite und verschiebt trotzdem die Indizes: Nach
# `calcination` kommen noch sechs Dateien.
#
# **Er war bis 2026-08-06 ein Verstoß und ist seitdem der normale Weg.** Was
# sich geändert hat, ist nicht das Verhalten, sondern die Bewertung: Der Wächter
# meldet die Verschiebung weiterhin, aber sie ist jetzt der Anlass,
# `spell-order.json` nachzuziehen — kein Verbot. Der fünfte Fall im Skript
# belegt, dass danach grün ist.
def einfuegen(modus):
    pfad = letzte_quelle() if modus == 'hinten' else 'data/spells/calcination.json'
    daten = json.loads(Path(pfad).read_text(encoding='utf8'))
    # Eine Kopie setzt alle Pflichtfelder; geprüft wird die Reihenfolge.
    neu = dict(daten['spells'][0])
    neu['id'] = 'gegenprobe-zauber'
    neu['name'] = 'Gegenprobe'
    if modus in {'hinten', 'eigene-datei'}:
        daten['spells'] = daten['spells'] + [neu]
    else:
        daten['spells'] = [neu] + daten['spells']
    Path(pfad).write_text(json.dumps(daten, ensure_ascii=False, indent=2), encoding='utf8')


if __name__ == '__main__':
    main()
