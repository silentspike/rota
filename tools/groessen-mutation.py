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
    # **Der Nullfall.** `0 // 2` ist wieder 0 — die Mutation griffe nicht, und
    # das Schweigen der Suite hieße gar nichts. Am 2026-08-07 hat die
    # Zusicherung unten den Lauf genau daran angehalten (`buff.kind` ist bei
    # allen fünf Verstärkern 0). Bei 0 wird deshalb hochgesetzt statt geteilt:
    # Die Richtung ist gleichgültig, verändert muss der Wert sein.
    if wert == 0:
        return 1
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
    """Welche DATEIEN rot werden — nicht bloß wie viele Fälle.

    **Die Zahl allein führt in die Irre**, und das ist am 2026-08-07 zweimal
    passiert. `flashpoint buff.factor 2 → 1` meldete „1 rot", was nach
    Absicherung aussah. Der eine rote Fall war `freshness.test.ts`: eine
    Änderungserkennung, die nur sagt, dass die eingecheckte Aufzeichnung nicht
    mehr zum Code passt. Ein `npm run parity:export` — der zum Arbeitsablauf
    gehört — macht sie wieder grün, **mit** dem halbierten Faktor. Gemessen:
    1777 grün, 0 rote Rust-Sammlungen.

    Eine Frische-Prüfung vergleicht die Aufzeichnung gegen den Code, der sie
    erzeugt hat. Gegen eine *unbeabsichtigte* Änderung hilft sie; gegen eine
    *absichtliche* Balance-Änderung ist sie wirkungslos. Das ist derselbe
    Fehler wie ein Sollwert aus derselben Quelle wie der Istwert, nur eine
    Ebene höher.

    Deshalb liefert diese Funktion die Dateinamen: Nur so ist
    „meldet sich mit einer Wirkungsprüfung" von „meldet sich nur über die
    Frische" zu unterscheiden.
    """
    # **stderr gehört dazu.** vitest schreibt die `FAIL`-Zeilen dorthin, die
    # Zusammenfassung nach stdout. Ein erster Entwurf las nur stdout — und
    # meldete daraufhin für JEDES Paar „SCHWEIGT", auch für slag, wo drei
    # eigene Fälle greifen. Ein Leser hätte daraus geschlossen, nichts sei
    # geprüft.
    lauf = subprocess.run(
        ['npx', 'vitest', 'run'],
        cwd=WURZEL,
        capture_output=True,
        text=True,
        timeout=900,
    )
    dateien = set()
    gemessen = False
    for zeile in (lauf.stdout + '\n' + lauf.stderr).splitlines():
        if zeile.startswith(' FAIL '):
            teil = zeile.split()[1].split('>')[0].strip()
            dateien.add(teil)
        if 'Tests ' in zeile and ('failed' in zeile or 'passed' in zeile):
            gemessen = True
    if not gemessen:
        return None  # Suite hat gar nicht gemessen — das ist KEIN Schweigen
    return dateien


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

            # Die drei Urteile, nach dem Befund vom 2026-08-07. Nur das erste
            # bedeutet Absicherung; die anderen beiden sehen wie eine aus.
            NUR_FRISCHE = {'tests/parity/freshness.test.ts'}
            if rot is None:
                urteil = 'KEINE MESSUNG'
            elif not rot:
                allein = ' [einziger Traeger -> #555]' if traeger[name] == 1 else ''
                urteil = f'SCHWEIGT{allein}'
            elif rot <= NUR_FRISCHE:
                urteil = 'NUR FRISCHE -> faktisch ungeschuetzt'
            else:
                echte = sorted(d.split('/')[-1] for d in rot - NUR_FRISCHE)
                urteil = f'geschuetzt ({", ".join(echte[:3])}{"..." if len(echte) > 3 else ""})'
            print(
                f'{name:26} {zauber_id:12} {vorher} -> {mutiere(vorher):<8} {urteil}',
                flush=True,
            )
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        print('# alle Datendateien zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
