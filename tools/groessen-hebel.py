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


GROESSEN = ('damage', 'damageTaken', 'absorbed', 'healed', 'converted')

# **`damageDealt` heißt anderswo `damage`.** Die vierte Bauart (`cases[].runs[]`,
# eingeführt mit `kontrolle-wirkt.json`) schreibt den ausgeteilten Schaden unter
# diesem Namen. Ohne die Zuordnung fiele er aus dem Vergleich — und die neue
# Lage, für die die Datei gebaut wurde, wäre unsichtbar.
GLEICHBEDEUTEND = {'damageDealt': 'damage'}


def laeufe_in(knoten):
    """Alle Läufe eines Aufzeichnungs-Baums — über ALLE vier Bauarten.

    ```text
      presses + expected   ein Lauf je Datei
      runs                 mehrere Läufe je Datei
      cases[].runs[]       Fälle mit je mehreren Läufen   (seit #593)
      anders               kein Kampflauf
    """
    if not isinstance(knoten, dict):
        return
    if 'cases' in knoten:
        for fall in knoten['cases']:
            yield from laeufe_in(fall)
        return
    if 'runs' in knoten:
        for lauf in knoten['runs']:
            yield from laeufe_in(lauf)
        return
    if 'expected' in knoten:
        yield knoten


def wirkungsbild():
    """Alle fünf Größen je LAUF — über alle drei Bauarten.

    **Nicht nur `damage`, und nicht nur eine Bauart.** Beides waren Fehler der
    ersten Fassung, und beide sahen wie Ergebnisse aus:

    - Der Schaden allein kann Heilungen, Schilde und Kontrolle nicht sehen.
      `salve` heilt, `plate` schirmt ab — eine Null im Schadenshebel ist dort
      die Definition des Zaubers, kein Befund (#576).
    - Die Bauart `presses + expected` ist nur eine von dreien. Wer über „alle
      Aufzeichnungen" misst, misst je nach Werkzeug 39, 43 oder 52.

    Der Schlüssel ist `datei#lauf`, weil die `runs`-Bauart mehrere Läufe je
    Datei führt — und genau dort liegen die Koagulations-Lagen.
    """
    bild = {}
    for pfad in sorted(AUFZ.glob('*.json')):
        roh = json.loads(pfad.read_text(encoding='utf8'))
        for n, lauf in enumerate(laeufe_in(roh)):
            # **Die Drücke gehören zum Bild.** Verschiebt eine Mutation sie,
            # misst der Hebel Größe UND Rotation zusammen — die Zahl ist dann
            # eine andere Aussage. Am 2026-08-07 hat das bei
            # `control.cost/anchor` eine 0,00 % als 6,67 % ausgegeben.
            #
            # Die Angabe steht JE ZEILE und nicht als Spalte je Datei: Die
            # Eigenschaft ist paarweise. Dieselbe Aufzeichnung kippt bei einer
            # Mutation und bei einer anderen nicht — drei Anläufe über
            # `payloads.ts` haben drei verschiedene Datei-Listen geliefert.
            erwartet = dict(lauf.get('expected', {}))
            for fremd, unser in GLEICHBEDEUTEND.items():
                if fremd in erwartet and unser not in erwartet:
                    erwartet[unser] = erwartet[fremd]
            werte = {g: erwartet[g] for g in GROESSEN if g in erwartet}
            if werte:
                werte['__druecke'] = json.dumps(lauf.get('presses', []))
                bild[f'{pfad.stem}#{n}'] = werte
    if not bild:
        raise RuntimeError('keine einzige Aufzeichnung mit Wirkungs-Größen')
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

    grund = wirkungsbild()

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
            jetzt = wirkungsbild()

            # Die größte relative Abweichung über ALLE Größen und ALLE Läufe —
            # und festgehalten, WELCHE Größe sich bewegt hat. Ohne diese
            # Angabe stünde in der Liste „0,00 %", wo in Wahrheit „gemessen
            # wurde die falsche Größe" gilt (#576 AC2).
            groesste, wo, welche = 0.0, '-', '-'
            basis, neuer = 0, 0
            verschoben = any(
                w.get('__druecke') != grund.get(k, {}).get('__druecke')
                for k, w in jetzt.items()
            )
            for lauf, werte in jetzt.items():
                for groesse, wert in werte.items():
                    if groesse == '__druecke':
                        continue
                    alt_wert = grund.get(lauf, {}).get(groesse, 0)
                    if not alt_wert:
                        continue
                    abweichung = abs(wert - alt_wert) / alt_wert
                    if abweichung > groesste:
                        groesste, wo, welche = abweichung, lauf, groesse
                        basis, neuer = alt_wert, wert
            # **Die Basis gehört daneben, sonst führt der Prozentsatz irre.**
            # 72,58 % bei searing sind ein Anteil am GESAMTSCHADEN; 300 % bei
            # stall ein Anteil an dem Teil, den er verhindert. Die Zahlen sind
            # nicht vergleichbar, und ohne die Basis sieht „300 vor 72" aus wie
            # „viermal so wichtig".
            print(
                f'{groesste * 100:6.2f}%  {name:26} {zauber_id:12} '
                f'{vorher} -> {neu_wert:<8} {welche:12} {basis} -> {neuer:<8} '
                f'{"Groesse+Rotation" if verschoben else "nur Groesse":16} {wo}',
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
