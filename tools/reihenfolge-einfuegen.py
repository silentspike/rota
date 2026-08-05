"""Fügt für die Gegenprobe (Issue #402) einen Zauber ein — vorne oder hinten.

Steht als eigene Datei neben dem Skript und nicht als Here-Dokument darin: Ein
eingebettetes Python-Fragment endet auf `PY`, und genau dieser Terminator
kollidiert mit dem Here-Dokument der Shell, die es einbettet.

Aufruf ausschließlich über `tools/reihenfolge-gegenprobe.sh` — nur dort werden
die Dateien vorher gesichert und danach zurückgestellt.
"""

import json
import sys

modus = sys.argv[1]

# Drei Fälle, und der dritte ist der lehrreiche (Issue #408):
#
#   vorne         an den Anfang der ERSTEN Quelle      -> Verstoß, offensichtlich
#   hinten        an das Ende der LETZTEN Quelle       -> erlaubt
#   eigene-datei  an das Ende von calcination.json     -> Verstoß, ÜBERRASCHEND
#
# Der dritte sieht aus wie der zweite und verhält sich wie der erste. Genau
# diese Lesart von „hinten" hat die Kommentare in `data.ts` nahegelegt, und
# genau sie verschiebt die Indizes: Nach `calcination` kommen noch sechs
# Dateien.
ANHAENGEN_ANS_ENDE = {'hinten', 'eigene-datei'}
pfad = (
    'data/spells/test-cast.json' if modus == 'hinten' else 'data/spells/calcination.json'
)

daten = json.load(open(pfad, encoding='utf8'))
# Eine Kopie des ersten Eintrags, damit alle Pflichtfelder gesetzt sind — der
# Zauber soll gültig sein, sonst bräche das Einlesen statt der Reihenfolge.
neu = dict(daten['spells'][0])
neu['id'] = 'gegenprobe-zauber'
neu['name'] = 'Gegenprobe'

if modus in ANHAENGEN_ANS_ENDE:
    daten['spells'] = daten['spells'] + [neu]
else:
    daten['spells'] = [neu] + daten['spells']
json.dump(daten, open(pfad, 'w', encoding='utf8'), ensure_ascii=False, indent=2)
