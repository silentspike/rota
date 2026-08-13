#!/usr/bin/env python3
"""Streich-Mutationen über die Wirkfelder der Zauberdaten (Orchestrator-Auftrag 6650).

DIE FRAGE: Ein Feld kann geparst und danach ignoriert werden. Dass sein Name in
sim/src vorkommt, belegt keine Wirkung — das war die Einschränkung meiner eigenen
Messung zu #513. Hier wird sie aufgehoben: Feld streichen, Prüfungen laufen lassen.
Färbt nichts, ist das Feld ungedeckt.

BLATTEBENE, NICHT GRUPPE. `buff` ist kein Feld, sondern vier (`buff.factor`,
`buff.gcds`, `buff.kind`, `buff.target`). Gemeinsam gestrichen sichern sie
einander nicht: Ein Lauf, der auf `buff.kind` rot wird, sagt nichts über
`buff.factor` — gemessen am 2026-08-11, wo 598 von 598 Prüfungen grün blieben,
während ein Nachbarfeld mutiert war.

DREI AUSGÄNGE, die auseinandergehalten werden müssen:
  ROT    mindestens eine Prüfung färbt  -> das Feld ist gedeckt
  GRUEN  alles bleibt grün              -> DAS ERGEBNIS: ungedeckt
  BAU    der Build bricht ab            -> build.rs verlangt das Feld; gedeckt,
         aber durch die Erzeugung, nicht durch eine Prüfung. Eigene Spalte, weil
         "cargo bricht ab" auch 0 rote Zeilen liefert und wie GRUEN aussieht.

Die Grundwerte `baseDamage`, `castTimeMs`, `cooldownMs`, `triggersGcd` stehen in
allen 57 Zaubern und sind Stammdaten, keine Wirkfelder — sie bleiben draußen.
"""

import argparse
import collections
import json
import pathlib
import signal
import subprocess
import sys
import time

WURZEL = pathlib.Path(__file__).resolve().parent.parent
SCHULEN = [
    "calcination",
    "coagulation",
    "common",
    "fixation",
    "putrefaction",
    "sublimation",
]
STAMM = {"id", "name", "school", "unlockLevel", "icon", "description", "_comment", "tooltip"}
GRUNDWERTE = {"baseDamage", "castTimeMs", "cooldownMs", "triggersGcd"}


def spells(schule):
    d = json.loads((WURZEL / f"data/spells/{schule}.json").read_text())
    return d if isinstance(d, list) else d["spells"]


def blattfelder():
    """Alle Wirkfelder als Pfad -> Menge der Zauber, die ihn tragen."""
    wo = collections.defaultdict(set)

    def geh(praefix, obj, sid):
        for k, v in obj.items():
            if k in STAMM or (not praefix and k in GRUNDWERTE):
                continue
            pfad = f"{praefix}{k}"
            if isinstance(v, dict):
                geh(pfad + ".", v, sid)
            else:
                wo[pfad].add(sid)

    for schule in SCHULEN:
        for s in spells(schule):
            geh("", s, s["id"])
    return wo


def streichen(obj, teile):
    """Entfernt den Pfad; gibt zurück, ob etwas entfernt wurde."""
    for t in teile[:-1]:
        if not isinstance(obj, dict) or t not in obj:
            return False
        obj = obj[t]
    return isinstance(obj, dict) and obj.pop(teile[-1], KeyError) is not KeyError


def rote_namen(ausgabe):
    """ALLE Namen der gefärbten Fälle, aus beiden Suiten-Formaten.

    Vollständig, nicht gekappt: Die Klassifikation "hat nur die Datenfrische
    gefärbt oder auch eine Wirkung?" entsteht erst aus dem Vergleich ÜBER alle
    58 Läufe. Ein Test, der bei jeder Streichung färbt, unterscheidet nichts —
    einer, der bei vieren färbt, benennt genau diese vier. Diese Auswertung ist
    gemessen; eine Liste "das sind die Frische-Tests" wäre geraten.
    """
    namen = []
    for zeile in ausgabe.splitlines():
        z = zeile.strip()
        if z.startswith("test ") and z.endswith("FAILED"):  # cargo
            namen.append(z[5:-11].strip())
        elif z.startswith("FAIL ") or z.startswith("×"):  # vitest
            namen.append(z.lstrip("FAIL ×").split(">")[0].strip())
    return list(dict.fromkeys(namen))


def schreibe(ziel, ergebnisse):
    zeilen = ["| Feld | Zauber | Ergebnis | Detail |", "| --- | ---: | --- | --- |"]
    zeilen += [f"| `{f}` | {n} | {s} | {d} |" for f, s, n, d in ergebnisse]
    ziel.write_text("\n".join(zeilen) + "\n")


def lauf(befehl, timeout):
    p = subprocess.run(befehl, shell=True, cwd=WURZEL, capture_output=True, text=True, timeout=timeout)
    return p.returncode, (p.stdout + p.stderr)


def main():
    # SIGTERM MUSS EINEN AUSGANG NEHMEN, der `finally` erreicht. Ohne diesen
    # Handler beendet das Signal den Prozess hart, die Sicherungskopie wird nie
    # zurückgeschrieben — und der mutierte Datensatz bleibt im Arbeitsbaum
    # liegen. Genau so passiert beim ersten Lauf dieses Skripts: Der
    # Bash-Timeout griff nach zehn Minuten, und `searing` stand danach ohne
    # `dot.tickDamage` im Baum. Jeder folgende Lauf hätte gegen kaputte Daten
    # gemessen, ohne dass etwas darauf hingewiesen hätte.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))

    ap = argparse.ArgumentParser()
    ap.add_argument("--feld", help="nur dieses Feld (Trägerprobe)")
    ap.add_argument("--ab", help="erst ab diesem Feld beginnen")
    ap.add_argument("--aus", default="docs/streich-mutation-ergebnis.md")
    ap.add_argument("--roh", default="/work/tmp/e1-mutation-namen.json",
                    help="alle roten Testnamen je Feld, Grundlage der Auswertung")
    ap.add_argument("--timeout", type=int, default=1800)
    args = ap.parse_args()

    # `--no-fail-fast` UND kein `--bail`, obwohl beides den Lauf vervielfacht.
    #
    # Der erste Anlauf sparte hier und war damit blind: Ein Snapshot-Test
    # (`freshness.test.ts`, "entspricht dem heutigen Spielstand") färbt bei
    # JEDER Datenänderung, ganz gleich ob das Feld wirkt. Bricht der Lauf beim
    # ersten roten Fall ab, sieht man nur ihn — und jedes der 58 Felder hieße
    # ROT, ohne dass eine einzige Wirkung geprüft worden wäre.
    #
    # Die Unterscheidung "Frische färbte" gegen "Wirkung färbte auch" braucht
    # die VOLLSTÄNDIGE Liste der roten Fälle. Deshalb laufen beide Suiten hier
    # bis zum Ende durch, und die Auswertung erfolgt über die Namen.
    rust = "cargo remote -c -- test --no-fail-fast --all-targets"
    ts = "npx vitest run"

    wo = blattfelder()
    # SELTENE FELDER ZUERST. Ein Feld, das 57 Zauber tragen, ist mit Sicherheit
    # gedeckt; die Frage stellt sich bei den Einzelstücken (`clearsDoom`,
    # `refundsInterruptedGcd`, `doublesNextCast`). Bricht der Lauf nach Stunden
    # ab, liegen dann die Antworten vor, auf die es ankommt.
    felder = sorted(wo, key=lambda f: (len(wo[f]), f))
    if args.feld:
        if args.feld not in wo:
            print(f"FEHLER: '{args.feld}' ist kein Wirkfeld. Bekannt sind {len(felder)}.")
            return 2
        felder = [args.feld]
    elif args.ab:
        felder = felder[felder.index(args.ab):]

    # NULLPROBE, bevor irgendetwas mutiert wird.
    #
    # Ohne sie misst der ganze Lauf gegen einen Baum, der schon rot ist — und
    # meldet jedes Feld als "gedeckt", weil ja etwas färbt. Genau so geschehen:
    # Ein uncommitteter Prüf-Boss (`test-lethal-cast.json` aus #472) ließ
    # `parity_boss_state` färben, weil der Simulator die Datei las und das Spiel
    # nicht. Dreizehn Felder standen bereits als ROT in der Tabelle, die
    # Träger-Zusicherung eingeschlossen — alle aus diesem Fremdgrund, keines
    # wegen der Streichung.
    #
    # Der Fremdgrund war dabei NICHT zu erkennen, ohne den roten Fall beim Namen
    # zu lesen: "eine Prüfung färbt" traf zu, die Zahl stimmte, der Exit-Code
    # stimmte. Nur der Name verriet es.
    print("Nullprobe (ohne Mutation) ...", flush=True)
    rc_r, _ = lauf(rust, args.timeout)
    rc_t, _ = lauf(ts, args.timeout)
    if rc_r != 0 or rc_t != 0:
        print(f"ABBRUCH: Der Baum ist schon vor der ersten Mutation rot "
              f"(rust={rc_r}, ts={rc_t}). Jedes Feld sähe gedeckt aus.")
        return 3
    print("Nullprobe gruen — gemessen wird gegen einen sauberen Baum.\n", flush=True)

    print(f"{len(felder)} Felder, je ein Lauf beider Suiten\n", flush=True)
    ergebnisse = []
    roh = {}
    for i, feld in enumerate(felder, 1):
        teile = feld.split(".")
        # Sicherungskopien ALLER Schulen, nicht nur der betroffenen: Ein Feld
        # kann in mehreren stehen (control.cost in fixation, heal.amount in
        # coagulation), und ein halb zurückgesetzter Datensatz verfälscht jeden
        # folgenden Lauf.
        pfade = {s: WURZEL / f"data/spells/{s}.json" for s in SCHULEN}
        sicherung = {s: p.read_text() for s, p in pfade.items()}
        getroffen = 0
        try:
            for schule, p in pfade.items():
                d = json.loads(p.read_text())
                liste = d if isinstance(d, list) else d["spells"]
                geaendert = False
                for s in liste:
                    if streichen(s, teile):
                        getroffen += 1
                        geaendert = True
                if geaendert:
                    p.write_text(json.dumps(d, indent=2, ensure_ascii=False) + "\n")

            # ZUSICHERUNG IN DEN DATENSATZ: Wurde überhaupt etwas gestrichen?
            # Ohne diese Zahl sieht ein Tippfehler im Pfad wie ein ungedecktes
            # Feld aus — der Lauf bleibt grün, weil er nichts geändert hat.
            if getroffen != len(wo[feld]):
                ergebnisse.append((feld, "FEHLER", getroffen, f"erwartet {len(wo[feld])}"))
                print(f"[{i}/{len(felder)}] {feld:<38} FEHLER: {getroffen} statt {len(wo[feld])} gestrichen", flush=True)
                continue

            t0 = time.monotonic()
            rc_r, aus_r = lauf(rust, args.timeout)
            gebrochen = "error: could not compile" in aus_r or "error[E" in aus_r
            rc_t, aus_t = lauf(ts, args.timeout)
            dauer = time.monotonic() - t0

            if gebrochen:
                stand, detail = "BAU", "Build bricht ab"
            elif rc_r != 0 or rc_t != 0:
                stand = "ROT"
                # DIE NAMEN, nicht die Zahl. "Eine Prüfung färbt" war auch dann
                # wahr, als die Ursache ein fremder Boss im Arbeitsbaum war —
                # Zahl, Exit-Code und Satz stimmten alle. Erst der Name
                # `parity_boss_state` zeigte, dass die Streichung damit nichts
                # zu tun hatte.
                namen = rote_namen(aus_r) + rote_namen(aus_t)
                roh[feld] = namen
                (WURZEL / args.roh).write_text(json.dumps(roh, indent=1, ensure_ascii=False))
                detail = f"{len(namen)} Faelle" if namen else "rot ohne benannten Fall"
            else:
                stand, detail = "GRUEN", "beide Suiten gruen"
            ergebnisse.append((feld, stand, getroffen, detail))
            print(f"[{i}/{len(felder)}] {feld:<38} {stand:<6} {getroffen:>2} Zauber  "
                  f"{detail}  ({dauer/60:.1f} min)", flush=True)
            # NACH JEDEM FELD schreiben, nicht erst am Ende: Ein Lauf über 62
            # Felder dauert Stunden, und ein Abbruch in Stunde drei darf nicht
            # alles Gemessene verwerfen.
            schreibe(WURZEL / args.aus, ergebnisse)
        finally:
            for schule, p in pfade.items():
                p.write_text(sicherung[schule])

    ziel = WURZEL / args.aus
    schreibe(ziel, ergebnisse)
    print(f"\n{ziel}")
    gruen = [f for f, s, _, _ in ergebnisse if s == "GRUEN"]
    print(f"UNGEDECKT: {len(gruen)} von {len(ergebnisse)}")
    for f in gruen:
        print(f"  {f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
