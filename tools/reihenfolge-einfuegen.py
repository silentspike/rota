"""Fügt für die Gegenprobe (Issue #402) einen Zauber ein — vorne oder hinten.

Steht als eigene Datei neben dem Skript und nicht als Here-Dokument darin: Ein
eingebettetes Python-Fragment endet auf `PY`, und genau dieser Terminator
kollidiert mit dem Here-Dokument der Shell, die es einbettet.

Aufruf ausschließlich über `tools/reihenfolge-gegenprobe.sh` — nur dort werden
die Dateien vorher gesichert und danach zurückgestellt.
"""

import json
import sys

vorne = sys.argv[1] == 'vorne'
# „vorne" muss in die ERSTE Quelle, „hinten" in die LETZTE. Beides in dieselbe
# Datei zu schreiben wäre falsch: calcination.json steht in der Mitte der
# Gesamtliste, ein Anhängen dort ist ein Einschub.
pfad = 'data/spells/calcination.json' if vorne else 'data/spells/test-cast.json'

daten = json.load(open(pfad, encoding='utf8'))
# Eine Kopie des ersten Eintrags, damit alle Pflichtfelder gesetzt sind — der
# Zauber soll gültig sein, sonst bräche das Einlesen statt der Reihenfolge.
neu = dict(daten['spells'][0])
neu['id'] = 'gegenprobe-zauber'
neu['name'] = 'Gegenprobe'

daten['spells'] = [neu] + daten['spells'] if vorne else daten['spells'] + [neu]
json.dump(daten, open(pfad, 'w', encoding='utf8'), ensure_ascii=False, indent=2)
