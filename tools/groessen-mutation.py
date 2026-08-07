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
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
DATEN = sorted((WURZEL / 'data' / 'spells').glob('*.json'))

# Ein Wert, der die Größe deutlich verändert, ohne die Struktur zu verletzen:
# halbieren trifft jede Zahl, und bei 1 landet man auf 0 statt im Bruch.
def mutiere(name, wert):
    """Ein anderer Wert für dieses Feld — bei Zahlen halbiert, bei Text getauscht."""
    if isinstance(wert, bool):
        return not wert  # `consumesAll`: die einzige sinnvolle Mutation eines Schalters
    if isinstance(wert, str):
        if name in TEXT_ALTERNATIVEN:
            # **Der erste UNGLEICHE.** Trägt ein Zauber den Ersatzwert bereits,
            # griffe die Mutation nicht — `control.effect` ist bei einem der
            # fünf Träger selbst `holdPhase`. Die Zusicherung in main() würde
            # den Lauf anhalten, statt still nichts zu messen.
            return next(a for a in TEXT_ALTERNATIVEN[name] if a != wert)
        # Kein gültiger anderer Wert vorhanden — siehe NUR_EIN_WERT oben.
        return 'rota-mutation-ungueltig'
    return mutiere_zahl(wert)


def mutiere_zahl(wert):
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


# **Textfelder steuern, WO eine Wirkung landet** (Issue #633). `buff.target`
# entscheidet, welcher Speicher den Verstärker bekommt; `control.effect`, was ein
# Kontroll-Zauber tut. Das ist dieselbe Sorte wie `buff.kind` — nur als Wort.
#
# Bis zum 2026-08-07 hat dieses Werkzeug sie übersprungen: Der Filter ließ nur
# Zahlen durch. Gemessen waren damit 24 Felder und 68 Paare.
#
# # Jede Alternative ist BELEGT, nicht ausgedacht
#
# Ein erster Entwurf dieser Tabelle enthielt `tick`, `extend`, `canker` und
# `blight` — Werte, die plausibel klangen und die ich hingeschrieben hatte, ohne
# nachzusehen. Drei davon gibt es nicht. Eine Mutation auf einen erfundenen Wert
# misst nicht die Größe, sondern die Fehlerbehandlung; schweigt die Suite, hieße
# der Befund „ungeprüft", obwohl gar nichts Gültiges eingesetzt wurde.
#
# Deshalb steht hinter jedem Eintrag seine Quelle. `tick` hat es zurück in die
# Tabelle geschafft — aber erst, nachdem `data.ts:93` es belegt hat.
TEXT_ALTERNATIVEN = {
    # Typunion in data.ts:93 — 8 gültige Werte, 6 im Bestand belegt.
    'buff.target': ['tick', 'damage'],
    # Fünf Werte im Bestand gemessen; jeder ist für jeden Träger gültig.
    'control.effect': ['holdPhase', 'slowCast'],
    # Andere ausgelieferte Zauber (von 50 gemessenen).
    'synergy.partner': ['searing', 'cinder'],
}

# **Diese drei tragen im ganzen Bestand nur EINEN Wert** — und ihr Typ lässt
# auch nur diesen zu (`data.ts:45`, `:298`). Es gibt keine gültige Alternative,
# also misst eine Mutation hier zwangsläufig etwas anderes: ob die
# Laufzeit-Prüfung den ungültigen Wert bemerkt. Das ist ein echter Schutz, aber
# ein struktureller — er sagt nichts darüber, ob die Größe selbst geprüft ist.
# Wer das Urteil „geschützt" für die beiden Fragen gleich liest, täuscht sich.
NUR_EIN_WERT = {'dot.refreshMode', 'convert.from', 'spender.resource'}


def messbar(feld, wert):
    """Trägt dieses Feld eine Größe oder eine Steuerung — oder nur Text für Menschen?

    **`_comment` ist ausgeschlossen.** Es steht in den Datendateien als
    Begründung für Leser und hat keine Wirkung; eine Mutation daran müsste
    schweigen, und dieses Schweigen wäre kein Befund, sondern die Definition
    eines Kommentars.
    """
    if feld.startswith('_'):
        return False
    # Ein Schalter ist eine Steuerung wie eine Zahl — `spender.consumesAll`
    # entscheidet, ob der ganze Vorrat draufgeht. Mutation: negieren.
    return isinstance(wert, (bool, int, float, str))


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
                    if not messbar(feld, wert):
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
    dateien = []
    gemessen = False
    for zeile in (lauf.stdout + '\n' + lauf.stderr).splitlines():
        if zeile.startswith(' FAIL '):
            teil = zeile.split()[1].split('>')[0].strip()
            dateien.append(teil)
        if 'Tests ' in zeile and ('failed' in zeile or 'passed' in zeile):
            gemessen = True
    if not gemessen:
        return None  # Suite hat gar nicht gemessen — das ist KEIN Schweigen
    return dateien


def main():
    # **`finally` allein reicht nicht.** Am 2026-08-07 lief ein Block in einen
    # Zeitüberlauf; der Abbruch kam als SIGTERM, und den behandelt Python nicht
    # als Ausnahme — das `finally` unten lief nie. Zurück blieb
    # `calcination.json` mit `consumesAll: false`, also mutierte Spieldaten im
    # Arbeitsbaum. Wer danach misst, misst gegen einen verfälschten Bestand,
    # ohne dass irgendetwas rot wird. Gefunden hat es nur ein `git status`.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit('# SIGTERM — stelle zurueck'))

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
            ziel[gruppe][feld] = mutiere(name, vorher)
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
            # **Die Spalte, die Worker-2s Frage beantwortet** (2026-08-07):
            # Wie viele ECHTE Prüffälle tragen diese Größe? Die rohe Fallzahl
            # taugt dafür nicht — `stay` und `quicksilver` haben beide zwei
            # rote Fälle, aber bei `stay` sind es zweimal die Frischeprüfung
            # und bei `quicksilver` einmal ein Fall, der die Größe kennt.
            # Wer roh zählt, hält die beiden für gleich breit getragen.
            #
            # Eine 1 hier heißt: Fällt dieser eine Fall weg, ist die Größe
            # wieder ungeschützt, und niemand merkt es. Eine 0 heißt: schon
            # jetzt niemand.
            echte_faelle = sum(1 for d in (rot or []) if d not in NUR_FRISCHE)
            traegt = f' [{echte_faelle} Prueffall]' if echte_faelle == 1 else (
                f' [{echte_faelle} Prueffaelle]' if echte_faelle else ''
            )
            rot = set(rot) if rot is not None else None
            if rot is None:
                urteil = 'KEINE MESSUNG'
            elif not rot:
                allein = ' [einziger Traeger -> #555]' if traeger[name] == 1 else ''
                urteil = f'SCHWEIGT{allein}'
            elif rot <= NUR_FRISCHE:
                urteil = 'NUR FRISCHE -> faktisch ungeschuetzt'
            else:
                echte = sorted(d.split('/')[-1] for d in rot - NUR_FRISCHE)
                # **Bei diesen Feldern heißt „rot" etwas anderes.** Sie haben
                # keine gültige Alternative, die Mutation setzt also einen
                # ungültigen Wert ein. Was anschlägt, ist die Struktur-Prüfung —
                # nicht ein Prüffall, der die Größe kennt. Ohne diesen Zusatz
                # stünde hier dasselbe Wort wie bei einem echt geprüften Feld.
                art = ' [STRUKTUR, nicht Groesse]' if name in NUR_EIN_WERT else ''
                urteil = (
                    f'geschuetzt{art}{traegt} '
                    f'({", ".join(echte[:3])}{"..." if len(echte) > 3 else ""})'
                )
            print(
                f'{name:26} {zauber_id:12} {vorher} -> {str(mutiere(name, vorher)):<8} {urteil}',
                flush=True,
            )
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        print('# alle Datendateien zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
