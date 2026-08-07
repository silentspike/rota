"""Misst, welche Größen-Felder im Zauber-Bestand nur auf EXISTENZ geprüft sind (Issue #547).

Das Verfahren ist die Mutation: einen Wert auf einen deutlich anderen setzen,
die volle Suite fahren, und protokollieren, ob sich etwas meldet. Schweigt
alles, ist die Größe dieses Feldes heute ungeprüft — eine Balance-Änderung
könnte den Zauber wirkungslos machen, ohne dass es auffällt.

    python3 tools/groessen-mutation.py 0 12      # Felder 0 bis 11
    python3 tools/groessen-mutation.py 12 24     # der Rest

Zwei Hälften, weil ein Lauf über alle 24 Felder rund zwanzig Minuten braucht.

# Die Zusicherung, ohne die das Ergebnis nichts wert wäre

Vor jeder Messung wird geprüft, dass die Mutation die Datei WIRKLICH verändert
hat (`assert vorher != nachher`). Ohne sie sähe ein Feld, dessen Mutation gar
nicht griff, exakt so aus wie ein ungeprüftes: Die Suite schweigt in beiden
Fällen.

Am 2026-08-06 ist genau das passiert — ein Suchmuster passte nach einer
Formatierung nicht mehr, der Aufruf lief ohne Fehler durch, und beide „Läufe"
maßen denselben Code. Gemeldet wurde daraufhin eine Lücke, die keine war.

# Warum die volle Suite und nicht die naheliegende Testdatei

„Ungeprüft" heißt: KEINE Prüfung meldet sich, nicht bloß die offensichtliche.
Ein Feld kann über eine Paritäts-Aufzeichnung abgesichert sein, ohne dass eine
Datei seinen Namen trägt.
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
DATEN = sorted((WURZEL / 'data' / 'spells').glob('*.json'))

# Ein Wert, der die Größe deutlich verändert, ohne die Struktur zu verletzen:
# halbieren trifft jede Zahl, und bei 1 landet man auf 0 statt im Bruch.
def mutiere(wert):
    if isinstance(wert, float):
        return round(wert / 2, 4)
    return max(0, wert // 2)


def felder():
    """Jedes PAAR (Zauber, Feld) über SPIELBARE Zauber.

    **Nicht je Feld einmal.** Am 2026-08-07 gemessen: `control.amountMs`
    schweigt bei `stall` und `latch`, meldet sich aber bei `stay` und
    `capstone` — zwei von vier Trägern desselben Feldes sind ungeschützt.

    Wer nur den ersten Träger misst, bekommt für dieses Feld eine Antwort, die
    für die Hälfte der Zauber falsch ist. Und zwar in beide Richtungen: Ein
    abgesicherter erster Träger verdeckt die Lücke dahinter, ein ungeschützter
    erklärt Zauber für ungeprüft, die es nicht sind.
    """
    gefunden = []
    for datei in DATEN:
        for zauber in json.loads(datei.read_text(encoding='utf8'))['spells']:
            if zauber.get('unlockLevel', 0) > 100:
                continue  # Prüf-Zauber, nicht ausgeliefert
            for gruppe, knoten in zauber.items():
                if not isinstance(knoten, dict):
                    continue
                for feld, wert in knoten.items():
                    if isinstance(wert, bool) or not isinstance(wert, (int, float)):
                        continue
                    gefunden.append(
                        (f'{gruppe}.{feld}', (datei, zauber['id'], gruppe, feld))
                    )
    return sorted(gefunden, key=lambda p: (p[0], p[1][1]))


def traegerzahl():
    """Wie viele spielbare Zauber jedes Feld überhaupt tragen.

    Für die Trennung der zwei Sorten Schweigen (Issue #555): Schweigt die
    Mutation bei einem Feld mit nur EINEM Träger, fehlt womöglich nicht der
    Prüffall — dann ist das Feld heute schlicht wirkungslos, weil es nichts
    gibt, wogegen es sich auswirken könnte.
    """
    zahl = {}
    for name, _ in felder():
        zahl[name] = zahl.get(name, 0) + 1
    return zahl


def suite_rot():
    """Zahl der roten Prüffälle in der VOLLEN Suite."""
    lauf = subprocess.run(
        ['npx', 'vitest', 'run'], cwd=WURZEL, capture_output=True, text=True, timeout=900
    )
    for zeile in reversed(lauf.stdout.splitlines()):
        if 'Tests ' in zeile and ('failed' in zeile or 'passed' in zeile):
            if 'failed' in zeile:
                return int(zeile.split('failed')[0].split()[-1])
            return 0
    return -1  # Suite hat gar nicht gemessen — das ist KEIN Schweigen


def main():
    von, bis = int(sys.argv[1]), int(sys.argv[2])
    alle = felder()
    traeger = traegerzahl()
    sicherung = Path(tempfile.mkdtemp(prefix='groessen-', dir='/work/tmp'))
    for d in DATEN:
        shutil.copy(d, sicherung / d.name)

    try:
        for name, (datei, zauber_id, gruppe, feld) in alle[von:bis]:
            roh = json.loads(datei.read_text(encoding='utf8'))
            ziel = next(s for s in roh['spells'] if s['id'] == zauber_id)
            vorher = ziel[gruppe][feld]
            ziel[gruppe][feld] = mutiere(vorher)
            neuer_text = json.dumps(roh, ensure_ascii=False, indent=2) + '\n'

            # **Die Zusicherung.** Greift die Mutation nicht, ist das Schweigen
            # der Suite bedeutungslos — siehe Dateikopf.
            assert neuer_text != datei.read_text(encoding='utf8'), f'{name}: Mutation griff nicht'
            assert ziel[gruppe][feld] != vorher, f'{name}: Wert unveraendert ({vorher})'

            datei.write_text(neuer_text, encoding='utf8')
            rot = suite_rot()
            shutil.copy(sicherung / datei.name, datei)

            if rot < 0:
                urteil = 'KEINE MESSUNG'
            elif rot == 0:
                # Ein Feld mit nur EINEM Träger, dessen Mutation schweigt, ist
                # womöglich gar nicht ungeprüft, sondern wirkungslos (#555).
                allein = ' [einziger Traeger -> #555 pruefen]' if traeger[name] == 1 else ''
                urteil = f'SCHWEIGT{allein}'
            else:
                urteil = f'{rot} rot'
            print(
                f'{name:26} {zauber_id:10} {vorher} -> {mutiere(vorher):<8} {urteil}',
                flush=True,
            )
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        print('# alle Datendateien zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
