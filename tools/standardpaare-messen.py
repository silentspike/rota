#!/usr/bin/env python3
"""Misst, ob jedes Standard-Feld jedes Zaubers von mindestens einem Fall gedeckt ist.

# Warum neben `groessen-mutation.py` und nicht darin

Jenes Werkzeug steigt in `felder()` nur in **verschachtelte** Gruppen ab und
misst damit 116 Paare; 309 sieht es nicht. Der Orchestrator hat ausdruecklich
verlangt, es unangetastet zu lassen — eine Aenderung daran waere eine
Entscheidung, die einmal fallen soll und nicht zweimal.

# Was gemessen wird

Fuer jedes Paar (Zauber, Feld) wird der Wert im Datensatz veraendert und die
Testsuite gefahren. Bleibt sie gruen, ist das Paar **ungedeckt**: Der Wert
koennte sich beliebig aendern, ohne dass ein Fall es bemerkt.

Die Mutation ist bewusst **klein und plausibel** — plus eins, nicht mal
tausend. Eine absurde Zahl faengt Fehler, die kein Mensch je macht; eine um
eins verschobene faengt die, die tatsaechlich passieren.

# Getrennt ausgewiesen: spielbar gegen test-*

Eine ungedeckte Groesse bei einem ausgelieferten Zauber bedeutet etwas anderes
als bei einem Pruef-Traeger. `test-*` existiert nur fuer Testfaelle; dort ist
eine Luecke oft harmlos. Bei den 57 spielbaren ist sie es nie.
"""

import json
import glob
import signal
import subprocess
import sys
import time

FELDER = ['unlockLevel', 'baseDamage', 'cooldownMs', 'castTimeMs', 'triggersGcd']


def mutiere(wert):
    """Kleine, plausible Veraenderung — der Typ entscheidet."""
    if isinstance(wert, bool):
        return not wert
    if isinstance(wert, (int, float)):
        return wert + 1
    return wert


def alle_paare():
    paare = []
    for pfad in sorted(glob.glob('data/spells/*.json')):
        roh = json.load(open(pfad))
        zauber = roh if isinstance(roh, list) else roh.get('spells', [])
        for i, s in enumerate(zauber):
            for f in FELDER:
                if f in s:
                    paare.append((pfad, i, s['id'], f))
    return paare


def testlauf():
    """True, wenn die Suite ROT ist — also das Paar gedeckt.

    **`--bail=1` halbiert die Laufzeit um mehr als das Zehnfache.** Gemessen an
    einem gedeckten Paar: 105 Sekunden ohne, 9 Sekunden mit. Der Grund ist die
    Verteilung — die meisten Paare sind gedeckt, und dort steht die Antwort
    nach dem ersten roten Fall fest. Nur die OFFENEN brauchen den vollen Lauf,
    weil erst sein Ende belegt, dass nichts rot wurde.

    Auf das Ergebnis wirkt es nicht: Ob eine Suite beim ersten oder beim
    zwanzigsten roten Fall abbricht, aendert nichts daran, DASS sie rot ist.
    """
    r = subprocess.run(['npx', 'vitest', 'run', '--bail=1'], capture_output=True, text=True)
    return r.returncode != 0


def main():
    # **SIGTERM in eine Ausnahme wandeln, damit `finally` laeuft.** Gemessen:
    # Ein `kill` auf den laufenden Messvorgang liess eine Mutation im Datensatz
    # zurueck — `calcination.json` trug danach einen verschobenen Wert, und der
    # naechste Testlauf war rot, ohne dass jemand etwas geaendert hatte. Ohne
    # diesen Griff schuetzt das `finally` nur gegen Ausnahmen, nicht gegen das
    # Abbrechen von aussen.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit('abgebrochen'))

    paare = alle_paare()
    von = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    bis = int(sys.argv[2]) if len(sys.argv) > 2 else len(paare)
    print(f"Paare gesamt: {len(paare)} — messe {von} bis {bis}", flush=True)

    for n, (pfad, index, spell_id, feld) in enumerate(paare[von:bis], start=von):
        original = open(pfad).read()
        daten = json.loads(original)
        liste = daten if isinstance(daten, list) else daten['spells']
        alt = liste[index][feld]
        liste[index][feld] = mutiere(alt)
        open(pfad, 'w').write(json.dumps(daten, indent=2, ensure_ascii=False) + "\n")

        begonnen = time.time()
        try:
            gedeckt = testlauf()
        finally:
            # **Immer zuruecksetzen, auch bei Abbruch.** Ein mutierter Datensatz,
            # der liegen bleibt, faelscht jede spaetere Messung.
            open(pfad, 'w').write(original)

        dauer = time.time() - begonnen
        marke = 'gedeckt' if gedeckt else 'OFFEN  '
        print(f"{n:4}/{len(paare)}  {marke}  {spell_id:22} {feld:14} "
              f"{alt} -> {mutiere(alt)}  ({dauer:.0f}s)", flush=True)


if __name__ == '__main__':
    main()
