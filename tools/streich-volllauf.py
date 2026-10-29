#!/usr/bin/env python3
"""Startet den vollen Streich-Mutationsdurchgang (#1055) mit einem Befehl.

    python3 tools/streich-volllauf.py --rolle <Rolle> [--stand origin/main]
                                      [--aus /work/tmp/rota-1055-…] [--trocken]
    python3 tools/streich-volllauf.py --rolle <Rolle> --probe FELD [--probe FELD …]

Mit `--probe` derselbe Weg für wenige Felder: Nullprobe, beide Vorproben, die
genannten Felder. Dann gilt Bedingung 1 umgekehrt — das Fenster muss FREI
sein, denn eine Probe ist kein zugeteilter Messlauf und darf keinen stören.

Der Durchgang fährt alle Wirkfelder, je Feld Remote-Bau, Remote-Rust-Suite und
lokale vitest-Suite, davor die Nullprobe und die zwei Vorproben. Er dauert
Stunden und braucht die Maschine für sich. Was dafür vorher stimmen muss, steht
nicht in einer Anleitung, sondern wird hier geprüft — jede fehlende Bedingung
endet mit Exit 2 und dem Grund, ohne dass etwas angelegt wurde:

  1. Das Messfenster ist der Rolle ZUGETEILT: `rota-fenster status` nennt sie
     als Inhaber, und es liegt keine Abgabe vor. Ein freies Fenster reicht
     nicht — es teilt der Orchestrator zu, nicht dieses Werkzeug.
  2. Der Stand ist ein Commit. Gemessen wird sein Inhalt in einem eigenen
     Checkout, nie der Arbeitsbaum (STREICH-MUTATION-ABNAHME.md).
  3. Das Ausgabeverzeichnis gibt es noch nicht — ein alter Nachweis darin
     sähe aus wie das Ergebnis dieses Laufs.
  4. Wo Ausgabe und Checkout landen, sind mindestens 1500 MB frei.
  5. `package-lock.json` des Standes gleicht dem der Quelle für node_modules;
     sonst wäre ein `npm ci` nötig, und das gehört nicht in dieses Werkzeug.

Danach: Checkout per `git worktree add --detach` unter
/work/rota-worktrees/<Rolle>-1055-volllauf (bzw. -probe), node_modules per Reflink,
der historische Bericht des Standes als Vergleichsgrundlage, und der Lauf
abgekoppelt (`setsid nohup`), weil ein Hintergrundlauf des Bash-Werkzeugs nach
Minuten ohne Meldung eingesammelt werden kann. Sein Exit-Code steht am Ende
des Protokolls als `rc=N`.

WARUM DER CHECKOUT UNTER /work/rota-worktrees LIEGT, und nicht neben den
Nachweisen unter /work/tmp: `rota-fenster` kennt Rollen nur als Verzeichnisse
dort und ordnet Prozesse über `rota-worktrees/<name>/` zu. Die erste Probe
(2026-10-28, 18:09 bis 18:41) lief unter /work/tmp, und um 18:25 wurde das
Fenster mitten hinein einer kerngepinnten Messung zugeteilt.

Exit 0: gestartet (oder mit --trocken: alle Bedingungen erfüllt). Exit 2:
nicht gestartet.
"""

import argparse
import datetime
import filecmp
import os
import pathlib
import shlex
import shutil
import subprocess
import sys

WURZEL = pathlib.Path(__file__).resolve().parent.parent
MINDEST_FREI_MB = 1500

# Wo der Checkout liegt, wenn `--baum` fehlt — dort, wo `rota-fenster` Rollen
# kennt (siehe oben). `ROTA_WORKTREES` ersetzt die Wurzel nur für Prüffälle: Sie
# liefen sonst gegen das, was gerade auf der Platte liegt. Am 29.10. lag der
# Checkout des echten Durchgangs dort, und drei Fälle wurden lokal rot, in der
# CI (ohne /work) nicht.
WORKTREES = pathlib.Path(os.environ.get("ROTA_WORKTREES") or "/work/rota-worktrees")


def fenster_inhaber():
    """(Exit-Code, Inhaber, Abgabe liegt vor?, Rohtext) aus `rota-fenster status`.

    `ROTA_FENSTER` ersetzt den Befehl durch einen absoluten Pfad. Über PATH
    allein ließe sich das nicht sicher tun: Scheitert der erste Treffer am
    Start, sucht der Kernel-Aufruf im nächsten Verzeichnis weiter und findet
    das echte Werkzeug — gemessen beim Bau dieser Prüffälle.
    """
    befehl = os.environ.get("ROTA_FENSTER", "rota-fenster")
    try:
        fertig = subprocess.run([befehl, "status"], capture_output=True, text=True,
                                timeout=60, check=False)
    except (OSError, subprocess.TimeoutExpired) as grund:
        return None, None, None, f"rota-fenster nicht ausführbar: {grund}"
    inhaber = abgabe = None
    for zeile in fertig.stdout.splitlines():
        schluessel, _, wert = zeile.partition(":")
        if schluessel.strip() == "Inhaber":
            inhaber = (wert.split() or [None])[0]
        elif schluessel.strip() == "Abgabe":
            abgabe = wert.strip() != "LIEGT NICHT VOR"
    return fertig.returncode, inhaber, abgabe, fertig.stdout + fertig.stderr


def platz(pfad):
    """(freie MB, geprüfter Ort) auf dem Dateisystem, das `pfad` aufnehmen wird.

    Geprüft wird der nächste vorhandene Vorfahr, nicht ein fester Ort: Ein
    festes `/work` gab es auf dem CI-Läufer nicht (FileNotFoundError, lint
    von PR #1160), und auf einer anderen Maschine wäre es das falsche
    Dateisystem gewesen.
    """
    ort = pfad
    while not ort.exists():
        ort = ort.parent
    return shutil.disk_usage(ort).free // (1 << 20), ort


def abbruch(text):
    print(f"NICHT GESTARTET: {text}")
    return 2


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--rolle", required=True, help="die Rolle, der das Messfenster zugeteilt ist")
    ap.add_argument("--stand", default="origin/main", help="zu messender Commit (Vorgabe origin/main)")
    ap.add_argument("--aus", help="Ausgabeverzeichnis, darf nicht existieren")
    ap.add_argument("--baum", help="Checkout, darf nicht existieren; Vorgabe "
                                   "/work/rota-worktrees/<Rolle>-1055-volllauf bzw. -probe")
    ap.add_argument("--node-modules-von", default="/work/rota",
                    help="Checkout, dessen node_modules per Reflink übernommen werden")
    ap.add_argument("--timeout", type=int, default=1800,
                    help="je Stufe und Versuch (nach Exit 75 neu), an streich-mutation.py")
    ap.add_argument("--trocken", action="store_true", help="nur prüfen und den Befehl zeigen")
    ap.add_argument("--probe", action="append", metavar="FELD",
                    help="nur diese Felder (mehrfach); verlangt ein FREIES Fenster")
    args = ap.parse_args()

    code, inhaber, abgabe, roh = fenster_inhaber()
    wer = f"Inhaber {inhaber}" if inhaber else "kein Inhaber"
    if args.probe:
        if code != 0:
            print(roh.rstrip())
            return abbruch(f"Das Messfenster ist belegt ({wer}, rc={code}) — eine Probe "
                           "läuft nur bei freiem Fenster")
    elif code != 1 or inhaber != args.rolle or abgabe is not False:
        print(roh.rstrip())
        return abbruch(f"Das Messfenster ist {args.rolle} nicht zugeteilt ({wer}, "
                       f"Abgabe {'liegt vor' if abgabe else 'liegt nicht vor'}, rc={code}). "
                       f"Zuteilen: rota-fenster geben {args.rolle} \"#1055 Volllauf\"")

    stand = subprocess.run(["git", "rev-parse", "--verify", "--quiet", f"{args.stand}^{{commit}}"],
                           cwd=WURZEL, capture_output=True, text=True, check=False)
    if stand.returncode != 0:
        return abbruch(f"kein Commit: '{args.stand}'")
    commit = stand.stdout.strip()

    art = "probe-" if args.probe else ""
    aus = pathlib.Path(args.aus or
                       f"/work/tmp/rota-1055-{art}{datetime.datetime.now():%Y%m%d-%H%M}").resolve()
    if aus.exists():
        return abbruch(f"{aus} existiert bereits — ein alter Nachweis darin sähe aus wie dieser Lauf")
    baum = pathlib.Path(args.baum or WORKTREES / f"{args.rolle}-1055-"
                        f"{'probe' if args.probe else 'volllauf'}").resolve()
    if baum.exists():
        return abbruch(f"Checkout {baum} existiert bereits — erst `git worktree remove {baum}`")
    plaetze = {platz(aus), platz(baum)}
    for frei_mb, ort in sorted(plaetze, key=lambda eintrag: str(eintrag[1])):
        if frei_mb < MINDEST_FREI_MB:
            return abbruch(f"nur {frei_mb} MB frei unter {ort}, verlangt {MINDEST_FREI_MB}")
    quelle = pathlib.Path(args.node_modules_von)
    lock = subprocess.run(["git", "show", f"{commit}:package-lock.json"], cwd=WURZEL,
                          capture_output=True, check=False)
    if lock.returncode != 0 or not (quelle / "node_modules").is_dir() \
            or (quelle / "package-lock.json").read_bytes() != lock.stdout:
        return abbruch(f"package-lock.json von {commit[:12]} gleicht nicht dem von {quelle}, "
                       "oder dort fehlt node_modules — dann wäre `npm ci` nötig")

    befehl = [sys.executable, "-B", "tools/streich-mutation.py", "--vorproben",
              "--aus", f"{aus}/result.md", "--roh", f"{aus}/failure-names.json",
              "--nachweis", f"{aus}/evidence.json", "--vergleich", f"{aus}/historical-report.md",
              "--bau-log", f"{aus}/build.log", "--timeout", str(args.timeout)]
    for feld in args.probe or []:
        befehl += ["--feld", feld]
    protokoll = aus / "lauf.log"
    huelle = ["setsid", "nohup", "sh", "-c",
              f"{shlex.join(befehl)}; echo \"rc=$?\" >> {shlex.quote(str(protokoll))}"]

    print(f"Messfenster: {'frei (Probe)' if args.probe else f'{args.rolle} (zugeteilt, keine Abgabe)'}")
    print(f"Stand:       {commit}  ({args.stand})")
    print(f"Ausgabe:     {aus}")
    for frei_mb, ort in sorted(plaetze, key=lambda eintrag: str(eintrag[1])):
        print(f"Platz:       {frei_mb} MB frei unter {ort} (verlangt {MINDEST_FREI_MB})")
    print(f"Checkout:    git worktree add --detach {baum} {commit}")
    print(f"node_modules: cp -a --reflink=always {quelle / 'node_modules'} {baum / 'node_modules'}")
    print(f"Befehl:      cd {baum} && {shlex.join(huelle)} > {protokoll} 2>&1")
    if args.trocken:
        print("\nTROCKEN: nichts angelegt, nichts gestartet.")
        return 0

    aus.mkdir(parents=True)
    subprocess.run(["git", "worktree", "add", "--detach", str(baum), commit], cwd=WURZEL,
                   check=True, capture_output=True)
    rest = subprocess.run(["git", "status", "--porcelain"], cwd=baum, capture_output=True,
                          text=True, check=True).stdout
    if rest.strip():
        return abbruch(f"der frische Checkout ist nicht sauber:\n{rest}")
    subprocess.run(["cp", "-a", "--reflink=always", str(quelle / "node_modules"),
                    str(baum / "node_modules")], check=True)
    if not filecmp.cmp(quelle / "package-lock.json", baum / "package-lock.json", shallow=False):
        return abbruch("package-lock.json im Checkout weicht ab")
    (aus / "historical-report.md").write_bytes(subprocess.run(
        ["git", "show", f"{commit}:docs/streich-mutation-ergebnis.md"], cwd=WURZEL,
        capture_output=True, check=True).stdout)
    # KEIN start_new_session: Dann wäre das Kind schon Gruppenführer, `setsid`
    # forkte noch einmal und beendete sich — die gespeicherte PID gehörte einem
    # Zombie (gemessen: Zustand Z nach 0,5 s; ohne den Schalter der laufende
    # Prozess). `setsid` allein gibt die eigene Sitzung.
    with protokoll.open("w") as log:
        lauf = subprocess.Popen(huelle, cwd=baum, stdout=log, stderr=subprocess.STDOUT,
                                stdin=subprocess.DEVNULL)
    (aus / "lauf.pid").write_text(f"{lauf.pid}\n")
    print(f"\nGESTARTET: PID {lauf.pid}. Verfolgen: tail -f {protokoll}")
    print(f"Fertig, wenn das Protokoll mit rc=N endet. Dann {aus}/evidence.json lesen")
    print("(completed, probes_passed, historical_comparison.full_pass).")
    if args.probe:
        print("Eine Probe ersetzt docs/streich-mutation-ergebnis.md NIE (full_pass bleibt false).")
    else:
        print("Erst danach docs/streich-mutation-ergebnis.md ersetzen und das Fenster abgeben:")
        print(f"  rota-fenster abgeben {args.rolle}")
    print(f"Der Checkout ist ein neuer Arbeitsbereich auf dem Bau-Dienst — danach aufräumen:")
    print(f"  git worktree remove {baum}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
