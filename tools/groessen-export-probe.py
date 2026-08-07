"""Zweite Stufe zu `groessen-mutation.py`: Beruhigt sich der rote Fall durch einen Neu-Export?

    python3 tools/groessen-export-probe.py control.cost:stall dot.tickDamage:cinder

# Warum es diese Stufe braucht

Die erste Stufe meldet, WELCHE Dateien rot werden. Daraus lässt sich am Namen
ablesen, ob `freshness.test.ts` dabei war — aber das Kriterium ist nicht der
Name. Es ist:

    Verstummt der Fall, wenn man die Aufzeichnungen neu exportiert?

Am 2026-08-07 gemessen, `flashpoint buff.factor 2 → 1`:

    vor  dem Export   Spiel 1 rot (freshness), Rust 2 rot (parity_buff_targets)
    nach dem Export   Spiel 1777 grün, Rust 0 rot — mit halbiertem Faktor

Beide Seiten verstummen. Die Rust-Fälle heißen nicht „freshness", tun aber
dasselbe: Sie vergleichen die eingecheckte Aufzeichnung gegen den Code, der
sie erzeugt hat. **Sie sichern, dass beide Seiten dasselbe rechnen, nicht dass
sie das Richtige rechnen.** Ein Neu-Export gehört zum Arbeitsablauf und ist
kein Warnsignal.

Deshalb misst diese Stufe die Eigenschaft selbst, statt sie am Dateinamen zu
raten.

# Warum nicht für jedes Paar

Zwei Läufe je Paar wären bei 68 Paaren 136 Läufe, dazu die Rust-Seite. Die
Paare mit 34 bis 50 roten Fällen brauchen es nicht: So viele Frische-Fälle
gibt es im Bestand nicht — bei ihnen ist mindestens einer eine echte
Wirkungsprüfung. Das ist eine **Begründung für die Auslassung**, keine
stillschweigende Lücke.
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent
DATEN = sorted((WURZEL / 'data' / 'spells').glob('*.json'))


def lauf(befehl, cwd=WURZEL, timeout=1200):
    return subprocess.run(befehl, cwd=cwd, capture_output=True, text=True, timeout=timeout)


def spiel_rot():
    e = lauf(['npx', 'vitest', 'run'])
    text = e.stdout + '\n' + e.stderr
    if 'Tests ' not in text:
        raise RuntimeError('vitest hat nicht gemessen')
    return {z.split()[1].split('>')[0].strip() for z in text.splitlines() if z.startswith(' FAIL ')}


def rust_rot():
    e = lauf(['cargo', 'remote', '-c', '--', 'test'], cwd=WURZEL / 'sim')
    text = (e.stdout + '\n' + e.stderr).replace('\r', '\n')
    if 'test result:' not in text:
        # **Die Meldung muss den Zustand zeigen, nicht nur behaupten.** Eine
        # erste Fassung sagte bloß „cargo test hat nicht gemessen" — und ließ
        # offen, ob der Bau brach, der Server fehlte oder das Suchmuster nicht
        # passte. Drei verschiedene Ursachen, eine Meldung.
        raise RuntimeError(
            f'cargo test hat nicht gemessen (rc={e.returncode}, '
            f'stdout {len(e.stdout)} B, stderr {len(e.stderr)} B)\n'
            f'--- stdout Ende ---\n{e.stdout[-800:]}\n'
            f'--- stderr Ende ---\n{e.stderr[-800:]}'
        )
    return sum(1 for z in text.splitlines() if 'test result: FAILED' in z)


def aufz_zurueck(sicherung, ziel):
    """Aufzeichnungen aus der KOPIE zurückstellen, nicht per `git checkout`.

    **Warum nicht checkout.** Der Orchestrator hat sich am 2026-08-07 mit
    `git checkout sim/src/balance.rs` sieben ungestagte Einfügungen
    weggeworfen und es erst am nächsten Lauf gemerkt: *checkout nimmt nicht
    die Mutation zurück, es nimmt die DATEI zurück.*

    Diese Probe exportiert die Aufzeichnungen absichtlich neu. Läge daneben
    eigene, noch nicht eingecheckte Arbeit in `recordings/`, risse ein
    `git checkout` sie mit. Die Kopie trifft nur, was die Probe selbst
    angefasst hat.
    """
    for datei in sicherung.glob('*.json'):
        shutil.copy(datei, ziel / datei.name)


def main():
    sicherung = Path(tempfile.mkdtemp(prefix='exportprobe-', dir='/work/tmp'))
    for d in DATEN:
        shutil.copy(d, sicherung / d.name)
    aufz = WURZEL / 'tests/parity/recordings'
    aufz_sicherung = Path(tempfile.mkdtemp(prefix='exportprobe-aufz-', dir='/work/tmp'))
    for a in aufz.glob('*.json'):
        shutil.copy(a, aufz_sicherung / a.name)

    try:
        for arg in sys.argv[1:]:
            name, zauber_id = arg.split(':')
            gruppe, feld = name.split('.')
            datei = next(
                d
                for d in DATEN
                if any(s['id'] == zauber_id for s in json.loads(d.read_text(encoding='utf8'))['spells'])
            )
            roh = json.loads(datei.read_text(encoding='utf8'))
            ziel = next(s for s in roh['spells'] if s['id'] == zauber_id)
            vorher = ziel[gruppe][feld]
            ziel[gruppe][feld] = round(vorher / 2, 4) if isinstance(vorher, float) else vorher // 2
            neu = json.dumps(roh, ensure_ascii=False, indent=2) + '\n'
            assert neu != datei.read_text(encoding='utf8'), f'{arg}: Mutation griff nicht'
            datei.write_text(neu, encoding='utf8')

            vor_spiel, vor_rust = spiel_rot(), rust_rot()
            lauf(['npm', 'run', 'parity:export'])
            nach_spiel, nach_rust = spiel_rot(), rust_rot()

            # Die eigentliche Frage: Ist NACH dem Export noch etwas rot?
            urteil = (
                'GESCHUETZT (ueberlebt den Export)'
                if nach_spiel or nach_rust
                else 'NUR AUFZEICHNUNG -> faktisch ungeschuetzt'
            )
            print(
                f'{arg:34} {vorher} -> {ziel[gruppe][feld]:<8} '
                f'vor: {len(vor_spiel)} Dateien / {vor_rust} Rust   '
                f'nach: {len(nach_spiel)} / {nach_rust}   {urteil}',
                flush=True,
            )

            for d in DATEN:
                shutil.copy(sicherung / d.name, d)
            aufz_zurueck(aufz_sicherung, aufz)
    finally:
        for d in DATEN:
            shutil.copy(sicherung / d.name, d)
        aufz_zurueck(aufz_sicherung, aufz)
        print('# Daten und Aufzeichnungen zurueckgestellt', flush=True)


if __name__ == '__main__':
    main()
