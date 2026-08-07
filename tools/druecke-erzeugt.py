"""Welche Aufzeichnungen bewegen ihre Drücke bei EINER BESTIMMTEN Mutation? — #576.

**Der Name der Frage ist wichtig, und dieses Werkzeug beantwortet die kleinere.**
Es misst nicht, ob eine Datei ihre Drücke aus einer Prioritätenliste erzeugt,
sondern ob *die hier gefahrene Mutation* sie verschiebt. Das ist nicht dasselbe:

    kindle.baseDamage 100 → 300   →  13 Dateien wandern, 40 nicht
    fixation.json                 →  steht unter „fest"

Und `fixation` erzeugt seine Drücke nachweislich neu — der Artefakt-Befund bei
`anchor.control.cost` beruht genau darauf. Es nutzt nur bind, stay, anchor und
stall; von KINDLEs Grundschaden ist seine Auswahl unberührt.

Die 40 „festen" heißen also: **von dieser einen Mutation unberührt.** Wie viele
davon bei einer anderen wandern, sagt dieser Lauf nicht.

Wer die größere Frage beantworten will, braucht je Datei eine Mutation an einem
Zauber, der in *ihren* `bindings` steht — mechanisch ableitbar, aber ein Export
je Datei statt einem.


    python3 tools/druecke-erzeugt.py

# Warum das über den Hebel entscheidet

`tools/groessen-hebel.py` misst „mutieren → exportieren → vergleichen". Wo der
Export die Drücke **neu erzeugt**, misst es die geänderte Rotation statt die
Größe.

Gemessen am 2026-08-07 an `anchor.control.cost`:

```
Gegenprobe: 0 rot VOR dem Export, 1 rot danach
fixation.json run1: presses nach dem Export ANDERS
```

Bei festen Drücken ändert `anchor.control.cost` am Schaden **nichts**. Die
6,67 % Hebel waren ein Artefakt.

# Warum je Datei und nicht je Paar

Die Eigenschaft hängt an der Aufzeichnung, nicht an der Größe: Entweder ihre
Payload-Funktion baut eine feste Liste, oder sie wählt je Takt aus. Für alle
Größen derselben Datei ist die Antwort dieselbe.

Deshalb genügt **eine** Mutation für den ganzen Bestand: Ändert sie irgendeine
Größe, die in die Auswahl eingeht, verschieben sich die Drücke aller erzeugten
Aufzeichnungen — die festen bleiben unberührt.
"""

import json
import shutil
import subprocess
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
AUFZ = WURZEL / 'tests' / 'parity' / 'recordings'
DATEN = sorted((WURZEL / 'data' / 'spells').glob('*.json'))


def druecke(pfad):
    """Alle Druckfolgen einer Datei, über alle vier Bauarten."""
    roh = json.loads(pfad.read_text(encoding='utf8'))
    gefunden = []

    def sammle(knoten):
        if not isinstance(knoten, dict):
            return
        if 'cases' in knoten:
            for f in knoten['cases']:
                sammle(f)
            return
        if 'runs' in knoten:
            for r in knoten['runs']:
                sammle(r)
            return
        if 'presses' in knoten:
            gefunden.append(knoten['presses'])

    sammle(roh)
    return gefunden


def main():
    sicherung = Path(tempfile.mkdtemp(prefix='druecke-', dir='/work/tmp'))
    for d in DATEN:
        shutil.copy(d, sicherung / d.name)
    aufz_sicherung = Path(tempfile.mkdtemp(prefix='druecke-a-', dir='/work/tmp'))
    for a in AUFZ.glob('*.json'):
        shutil.copy(a, aufz_sicherung / a.name)

    vorher = {p.name: druecke(p) for p in sorted(AUFZ.glob('*.json'))}

    try:
        # **KINDLEs Grundschaden.** Er geht in jede Auswahl ein, die nach dem
        # stärksten Zauber greift — eine Mutation, die möglichst viele
        # erzeugte Folgen bewegt. Eine Größe, die keine Auswahl beeinflusst,
        # würde nichts trennen und alle Dateien als „fest" ausweisen.
        datei = next(d for d in DATEN if 'calcination' in d.name)
        roh = json.loads(datei.read_text(encoding='utf8'))
        ziel = next(s for s in roh['spells'] if s['id'] == 'kindle')
        alt = ziel['baseDamage']
        ziel['baseDamage'] = alt * 3
        text = json.dumps(roh, ensure_ascii=False, indent=2) + '\n'
        assert text != datei.read_text(encoding='utf8'), 'Mutation griff nicht'
        datei.write_text(text, encoding='utf8')

        lauf = subprocess.run(
            ['npm', 'run', 'parity:export'], cwd=WURZEL, capture_output=True, text=True, timeout=900
        )
        if lauf.returncode != 0:
            raise RuntimeError(f'Export fehlgeschlagen: {lauf.stderr[-400:]}')

        erzeugt, fest = [], []
        for pfad in sorted(AUFZ.glob('*.json')):
            (erzeugt if druecke(pfad) != vorher[pfad.name] else fest).append(pfad.stem)

        print(f'kindle.baseDamage {alt} -> {alt * 3}\n')
        print(f'ERZEUGTE Drücke ({len(erzeugt)}): der Hebel misst Größe UND Rotation')
        for n in erzeugt:
            print(f'  {n}')
        print(f'\nFESTE Drücke ({len(fest)}): der Hebel misst die Größe')
        for n in fest:
            print(f'  {n}')
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        for a in aufz_sicherung.glob('*.json'):
            shutil.copy(a, AUFZ / a.name)
        print('\n# Daten und Aufzeichnungen zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
