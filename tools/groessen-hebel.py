"""Misst, wie stark jede Größe den Schaden bewegt — die Rangfolge für #547 AC3.

    python3 tools/groessen-hebel.py 0 10

# Warum gemessen und nicht geschätzt

Die Frage lautet: Welche der 45 ungeschützten Größen bekommt zuerst einen
Wirkungs-Fall? Eine Rangfolge nach Feldnamen wäre geraten — `dot.tickDamage`
*klingt* wichtiger als `control.cost`, aber das ist eine Vermutung über den
Kampfverlauf, keine Messung. Gerade in einem Issue, dessen ganzer Inhalt
lautet „prüft die Größe, nicht die Existenz", wäre eine geschätzte Rangfolge
der falsche Abschluss.

# Wie

Jede Aufzeichnung unter `recordings/` trägt in `expected.damage` den Schaden
eines vollständigen Laufs. `npm run parity:export` schreibt sie neu aus dem
aktuellen Code. Also:

    Wert halbieren → exportieren → expected.damage aller Aufzeichnungen
    gegen den Ausgangsstand vergleichen

Die größte relative Abweichung über alle Aufzeichnungen ist der Hebel. Das
braucht **keinen** Testlauf — nur den Export, der ohnehin die Zahlen erzeugt,
die das Spiel selbst rechnen würde.

Ein Paar mit Hebel 0 % bewegt in keiner heutigen Aufzeichnung Schaden. Das ist
kein Widerspruch zu „ungeschützt", sondern die Erklärung: Wo nichts sich
bewegt, kann auch keine Prüfung anschlagen (die Sorte „keine Lage").
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
DATEN = sorted((WURZEL / 'data' / 'spells').glob('*.json'))
AUFZ = WURZEL / 'tests' / 'parity' / 'recordings'


def schadensbild():
    """`expected.damage` je Aufzeichnung — soweit vorhanden.

    **Nicht jede Aufzeichnung führt Schaden.** Ein erster Entwurf griff hart
    auf `['expected']['damage']` zu und brach mit `KeyError` ab. Der Abbruch
    war die richtige Reaktion: Hätte die Funktion stattdessen 0 geliefert,
    stünde für jede solche Aufzeichnung eine Abweichung von 0 % in der
    Rangfolge — nicht als „kein Schaden gemessen", sondern als „kein Hebel".
    """
    bild = {}
    for p in sorted(AUFZ.glob('*.json')):
        erwartet = json.loads(p.read_text(encoding='utf8')).get('expected', {})
        if 'damage' in erwartet:
            bild[p.name] = erwartet['damage']
    if not bild:
        raise RuntimeError('keine einzige Aufzeichnung mit expected.damage')
    return bild


def exportieren():
    e = subprocess.run(
        ['npm', 'run', 'parity:export'],
        cwd=WURZEL,
        capture_output=True,
        text=True,
        timeout=900,
    )
    if e.returncode != 0:
        raise RuntimeError(f'Export fehlgeschlagen (rc={e.returncode})\n{e.stderr[-600:]}')


def main():
    von, bis = int(sys.argv[1]), int(sys.argv[2])
    zeilen = [z.split() for z in (WURZEL / 'docs/groessen-abdeckung-messung.txt').read_text().splitlines()]
    # Nur die ungeschützten — für die geschützten ist die Rangfolge gegenstandslos.
    offen = [(z[0], z[1]) for z in zeilen if 'SCHWEIGT' in z or 'FRISCHE' in ' '.join(z)]

    sicherung = Path(tempfile.mkdtemp(prefix='hebel-', dir='/work/tmp'))
    for d in DATEN:
        shutil.copy(d, sicherung / d.name)
    aufz_sicherung = Path(tempfile.mkdtemp(prefix='hebel-aufz-', dir='/work/tmp'))
    for a in AUFZ.glob('*.json'):
        shutil.copy(a, aufz_sicherung / a.name)

    grund = schadensbild()

    try:
        for name, zauber_id in offen[von:bis]:
            gruppe, feld = name.split('.')
            datei = next(
                d
                for d in DATEN
                if any(
                    s['id'] == zauber_id
                    for s in json.loads(d.read_text(encoding='utf8'))['spells']
                )
            )
            roh = json.loads(datei.read_text(encoding='utf8'))
            ziel = next(s for s in roh['spells'] if s['id'] == zauber_id)
            vorher = ziel[gruppe][feld]
            neu_wert = 1 if vorher == 0 else (round(vorher / 2, 4) if isinstance(vorher, float) else vorher // 2)
            ziel[gruppe][feld] = neu_wert
            text = json.dumps(roh, ensure_ascii=False, indent=2) + '\n'
            assert text != datei.read_text(encoding='utf8'), f'{name}: Mutation griff nicht'
            datei.write_text(text, encoding='utf8')

            exportieren()
            jetzt = schadensbild()

            groesste = 0.0
            wo = '-'
            for aufzeichnung, schaden in jetzt.items():
                alt = grund.get(aufzeichnung, 0)
                if alt:
                    abw = abs(schaden - alt) / alt
                    if abw > groesste:
                        groesste, wo = abw, aufzeichnung.replace('.json', '')
            print(
                f'{groesste * 100:6.2f}%  {name:26} {zauber_id:12} '
                f'{vorher} -> {neu_wert:<8} {wo}',
                flush=True,
            )

            for d in DATEN:
                shutil.copy(sicherung / d.name, d)
            for a in aufz_sicherung.glob('*.json'):
                shutil.copy(a, AUFZ / a.name)
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        for a in aufz_sicherung.glob('*.json'):
            shutil.copy(a, AUFZ / a.name)
        print('# Daten und Aufzeichnungen zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
