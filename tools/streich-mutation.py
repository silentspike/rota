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

VIER AUSGÄNGE, die auseinandergehalten werden müssen:
  ROT     ausgeführte Prüfungen färben  -> Herkunft getrennt in Rust und TS
  GRUEN   beide Suiten bleiben grün    -> diese Mutation wurde nicht bemerkt
  BAU     der Rust-Bau bricht ab       -> keine Rust-Prüfung; TS separat lesen
  FEHLER  Suite leer oder abgebrochen  -> kein Deckungsnachweis

BAU ist kein Nachweis der Wertdeckung. Auch GRUEN beantwortet die Deckungsfrage
erst mit einer wirksamen Gegenrichtung; siehe die beiden Modi weiter unten.

Die Grundwerte `baseDamage`, `castTimeMs`, `cooldownMs`, `triggersGcd` stehen in
den Zaubern und sind Stammdaten, keine Wirkfelder — sie bleiben draußen.

═══════════════════════════════════════════════════════════════════════════════
 ZWEI MODI, ZWEI VERSCHIEDENE FRAGEN — und sie werden leicht verwechselt (#1087)
═══════════════════════════════════════════════════════════════════════════════

    ohne Schalter    Feld STREICHEN      fragt: "ist das Feld PFLICHT?"
    --delta N        Wert ÄNDERN         fragt: "ist der WERT gedeckt?"

**Die Streichung kann die zweite Frage nicht beantworten.** Jedes Feld, das
`build.rs` mit einem UNBEWACHTEN `uint()` liest, bricht beim Entfernen IMMER
den Bau — ganz gleich, ob irgendeine Prüfung seinen Wert ansieht. Die Spalte
meldet dann `BAU`, und `BAU` heißt nur „Pflichtfeld", nicht „gedeckt".

**Es sind DREI Muster, nicht zwei** — und das mittlere ist die Kategorie, die
beim ersten Lesen niemand gesehen hat:

                        bricht der Bau?    AUFRUFE   Feldnamen
    uint() unbewacht    ja                      62          40
    uint() BEWACHT      nein                     1           1
    opt_uint()          nein                    12          11

Die Einheit steht in der Überschrift, nicht in der Zelle — der erste Entwurf
zählte in den ersten zwei Zeilen Aufrufe und in der dritten Feldnamen. Wer ihn
addierte, bekam 74 und meinte, etwas gezählt zu haben. **Das ist genau die
Klasse, die dieser Dateikopf beschreibt, in ihm selbst.**

**Jede dieser Zahlen braucht ihre Definition daneben**, sonst widersprechen
sie einander morgen. Für `uint()` gibt es vier richtige Antworten:

    65   Regex-Treffer auf `uint(`                      einschl. der Definition
    64   echte Aufrufe                                  ohne `fn uint(…)`
    63   Aufrufe mit LITERALEM Feldnamen                <- die brauchbare Zahl
    41   verschiedene Feldnamen                         Aufrufe mehrfach gezählt

Die 63 ist die brauchbare, weil nur ein literaler Feldname sich einem
Tabellenfeld zuordnen lässt. Der eine Ausreißer steht in `build.rs:2168`:

    let amount = uint(node, feld, &whose);

`feld` ist dort eine Variable und je nach `control.effect` `"amountMs"`,
`"amountPercent"` oder `"amountPerPoint"`. **Ein Aufruf, drei Tabellenfelder**
— und der Grund, warum `control.amountPercent` als Pflichtfeld auftritt,
obwohl es anderswo mit `opt_uint` gelesen wird.

„62 unbewacht" oben heißt also: literale Aufrufe minus den einen bewachten.
Eine Zahl ohne diesen Satz ist in vier Wochen ein Widerspruch.

**Eine VIERTE Art, den Bau zu brechen — und sie trifft `--delta` selbst.**
Direkt hinter der Auswahlstelle steht:

    assert!(amount > 0, "{whose}: {feld} muss groesser als null sein");

Eine Wertänderung, die auf 0 oder darunter führt, bricht den Bau also
ebenfalls — aber wegen der Zusicherung, nicht wegen `uint()`. In der Tabelle
stünde `BAU`, und das hieße dort weder „Pflichtfeld" noch „ungedeckt",
sondern „unzulässiger Wert".

Wer `--delta` mit einem negativen Wert über die Tabelle laufen lässt, muss
damit rechnen. Bei einem Feld mit Wert 1 genügt `--delta -1`.

Das eine bewachte steht in `build.rs:820`:

    let decay_per_tick = if dot.get("decayPerTick").is_some() {
        uint(dot, "decayPerTick", &whose)

Der Aufruf kommt nur, wenn das Feld da ist. Beim Streichen bricht nichts, die
Rust-Suite läuft, sie färbt — `dot.decayPerTick` trug in den Rohdaten vom
13.08. denn auch 4 rustartige Namen von 22.

**Warum das eigens dasteht:** Ein bewachtes `uint()` taucht in einer Suche
nach `uint` auf und in einer nach `opt_uint` **nicht**. Wer die Kategorien
über Namen sucht, sortiert es in die falsche Gruppe — genau das ist beim
Schreiben dieses Modus passiert, in beide Richtungen: Erst zählte ein Regex
`uint(` die elf `opt_uint`-Felder mit (weil `opt_uint` auf `uint(` endet),
dann erklärte die Korrektur fünf von sechs Fällen und nahm den sechsten
mit. Eine Erklärung, die fünf Fälle deckt, ist nicht dieselbe wie eine, die
alle deckt.

Wer das verwechselt, liest die Tabelle falsch. Genau so standen dort 33 Felder
als gedeckt, die es nicht waren.

    Feld mit uint()-Lesestelle, gestrichen   -> BAU    (sagt nichts über den Wert)
    dasselbe Feld, Wert 40 -> 41             -> ROT    (eine Prüfung sieht ihn)
                                             -> GRÜN   (keine tut es)

**`--delta` hat seine eigene Falle, und sie ist AC3 des Issues:** Eine Änderung
um 1 kann WIRKUNGSLOS sein. Trägt der Wert eine Schwelle und liegt 41 noch
diesseits davon, meldet der Lauf `GRÜN` für ein Feld, das sehr wohl gedeckt
ist. Eine Mutation, die den Gegenstand nicht angreifen kann, belegt nichts.

Deshalb druckt der Modus bei jedem Feld, was er TATSÄCHLICH geändert hat
(`spell-id: alt->neu`). Ohne diese Zeile ist ein `GRÜN` nicht deutbar — man
sieht nicht, ob 15 zu 16 wurde (derselbe Fall, knapp daneben) oder 0 zu 1 (bei
einer Aufzählung wie `buff.kind` ein völlig anderer Fall).

**Nicht jede Zahl ist eine Größe.** `buff.kind` trägt die Werte 0 und 1 — das
ist eine Aufzählung, keine Menge. Ein `+1` darauf prüft nicht „ist der Wert
gedeckt", sondern schaltet auf einen anderen Fall um, und bei `1 + 1 = 2` womöglich
auf gar keinen. Wer die Tabelle über alle Zahlenfelder laufen lässt, muss diese
Unterklasse getrennt lesen.

**Und die EINHEIT entscheidet, ob `+1` überhaupt etwas bewirken kann:**

    Aufzählung      `+1` wechselt den FALL             buff.kind [0,1]
    feine Größe     `+1` kann im Raster verschwinden   extendShieldMs (ms)
    grobe Größe     `+1` wirkt immer                   dotExtendTicks (Ticks)

Gemessen an `extendShieldMs` (Wert 12000): `combat.ts` addiert dort ohne
`snapUp` und vergleicht gegen `seitBeginn`, das nur Vielfache von `TICK_MS`
(10) annimmt. 12000 und 12001 laufen **beide erst bei 12010** ab — die
Änderung verschwindet. Bei `dotExtendTicks` dagegen verschiebt ein Tick um
1000 oder 2000 ms, also um 100 bis 200 Takte.

**Dass `extendShieldMs` diese Klasse überhaupt vorführen kann, ist ein
Sonderfall — und der Grund dafür ist ein Befund über den Bestand.** Fast alle
`Ms`-Felder brechen bei einem Schritt neben dem Raster den Bau, bevor
irgendetwas läuft: `build.rs` erzwingt `is_multiple_of(RASTER_MS)` an **20**
Stellen (`atMs`, `perStepMs`, `castTimeMs`, `windowMs`, `overMs`,
`tickIntervalMs`, `amountMs`, fünf `durationMs` und weitere).

`extendShieldMs` und `extendDecayMs` gehören zu keiner davon — in **keiner**
der beiden Sprachen:

    sim/build.rs:1862   spell.get("extendShieldMs")
                            .and_then(Value::as_u64).unwrap_or(0)
                        kein uint(), kein assert!, keine Rasterprüfung —
                        und bei fehlender Angabe still 0

    src/core/data.ts:1741   wert <= 0            -> Meldung
                            !Number.isInteger    -> Meldung
                            sonst nichts

Ein `extendShieldMs` von 12001 kommt heute durch Bau und Prüfung und **wirkt
als 12010**. Ob das Absicht ist, steht in keinem Kommentar an den beiden
Lesestellen; an den 20 anderen hat jemand entschieden, dass es ein Fehler ist.
Das ist eine Design-Frage und wurde als solche gemeldet, nicht hier
entschieden.

**WAS SICH DARAUS NICHT ABLEITEN LÄSST: eine automatische Einteilung.** Ein
Versuch, die 45 Zahlenfelder nach ihren Werten zu sortieren („höchstens zwei
verschiedene, alle unter vier → Aufzählung"), hat sechs Felder falsch
einsortiert — darunter `dotExtendTicks [1]`, `decayCap.bonus [2]` und
`cooldownResetCharges [1,2]`. Ein Feld mit dem Wert 1 ist nicht deshalb eine
Aufzählung. Der Unterschied steht in der Verwendung, nicht in der
Werteverteilung, und wer nach Werten sortiert, misst das Leichte statt des
Nötigen.

**Der Prüfschritt statt der Einteilung**, und er gilt je Feld:

> Liegt eine Entscheidung GENAU auf dem Wert? Bei `decayWard.durationMs`
> (6000) ja — der Prüffall setzt einen Schlag exakt auf 12000, und `<=`
> entscheidet dort. Bei `extendShieldMs` nein. Wenn nein, ist `+1`
> wahrscheinlich wirkungslos, und ein `GRÜN` heißt nichts.

Sechzehn der 45 Zahlenfelder tragen die Endung `Ms`. Für sie ist diese Frage
Pflicht, bevor ein `GRÜN` als „ungedeckt" in die Tabelle geht.

**Nicht die GRÖSSE der Änderung entscheidet, sondern ob sie eine Grenze
überschreitet.** Der Beleg steht in `decayWard.durationMs` (Wert 6000) und
seinem Prüffall `tests/unit/putrefaction-fenster.test.ts:182` — „der Schlag bei
12000 ms liegt gerade noch darin". Die Logik dazu:

    combat.ts:2485   decayWardUntilMs = seitBeginn + durationMs
    combat.ts:3412   seitBeginnFuerDecay <= decayWardUntilMs

Daraus folgt, ohne einen Lauf:

    6000 -> 6001    Fenster bis 12001, Schlag bei 12000
                    12000 <= 12001   -> weiter drin   WIRKUNGSLOS
    6000 -> 5999    Fenster bis 11999
                    12000 <= 11999   -> draußen       WIRKSAM

Ein Feld, zwei gleich große Änderungen, entgegengesetzte Ausgänge. Wer nur
`+1` fährt, bekommt hier `GRÜN` für ein Feld, dessen Wert sehr wohl geprüft
wird — und schreibt „ungedeckt" in die Tabelle.

*Diese Vorhersage steht hier, bevor der Lauf sie prüft.* Ein Ergebnis, das
erst hinterher zur Erklärung wird, ist keine Prüfung.

**DER LAUF HAT SIE FALSIFIZIERT, und zwar mit einem dritten Ausgang.** Am
2026-08-22 gefahren, `--feld decayWard.durationMs --delta 1`:

    decay: 6000->6001     BAU     Build bricht ab     0,6 min

    sim/build.rs:1994, im Wortlaut aus der Bauausgabe:
    calcination.json/spells[decay]/decayWard: durationMs 6001 liegt nicht
    auf dem 10-ms-Raster

Die Rechnung oben ist richtig und **kommt nie zur Anwendung** — der Wert
erreicht den Simulator gar nicht. `build.rs` erzwingt das Raster mit
`duration.is_multiple_of(RASTER_MS)`, und `5999` bricht aus demselben Grund.
Beide Richtungen von AC3 sind mit `±1` an diesem Feld **nicht durchführbar**.

**Die Schrittweite muss auf dem Raster liegen, sonst misst man den Bau statt
der Deckung.** Mit `±10` bleibt der Kern von AC3 vollständig erhalten — ein
Feld, zwei gleich große Änderungen, entgegengesetzte Ausgänge:

    6000 -> 6010    Fenster bis 12010, Schlag bei 12000  -> drin
    6000 -> 5990    Fenster bis 11990, Schlag bei 12000  -> draußen

Damit hat `--delta` bei gerasterten `Ms`-Feldern eine eigene Vorbedingung, die
in keiner der drei Einheiten-Klassen steht: **Ist der Schritt selbst
zulässig?** Ein Schritt neben dem Raster erzeugt `BAU` — und `BAU` sagt nichts
über die Deckung des Wertes, nur über die Zulässigkeit der Mutation.

**`+10` gefahren, und das Ergebnis ist der Fall, vor dem dieser Modus warnt:**

    decay: 6000->6010    GRÜN    beide Suiten grün    3,2 min
                                 Werkzeug schreibt „UNGEDECKT: 1 von 1"

Das Feld ist **nicht** ungedeckt. Der Prüffall steht direkt darauf — er sieht
nur diese Richtung nicht: 12000 <= 12010 lässt den Schlag weiter hinein.
`--delta -10` färbt dieselbe Zeile.

**Daraus folgen VIER Ausgänge, von denen zwei gleich aussehen:**

    BAU     die Änderung ist unzulässig     gerastertes Feld, ±1
    GRÜN    die Änderung ist wirkungslos    decayWard.durationMs, +10
    GRÜN    das Feld ist ungedeckt          der gesuchte Fall
    ROT     das Feld ist gedeckt

Die beiden `GRÜN` trennt kein Lauf, sondern nur die **Gegenrichtung**: Wird
dasselbe Feld in die andere Richtung rot, war das `GRÜN` Wirkungslosigkeit.
Bleibt es auch dort grün, ist der Wert wirklich ungedeckt. Ein einzelner
`--delta`-Lauf kann „ungedeckt" nicht belegen — er kann es nur behaupten.

**`build.rs` wird NICHT angefasst.** Man könnte `uint()` durch `opt_uint(…, 0)`
ersetzen, dann bräche der Bau nicht mehr. Das tauschte eine härtere Zusicherung
gegen eine weichere — und eine Zahl, die bei fehlender Angabe still auf 0
fällt, sieht in jeder Messung aus wie eine gesetzte.

**DER TYP MUSS ERHALTEN BLEIBEN, und das ist keine Sauberkeit.** `uint()`
lehnt Fließkomma ausdrücklich ab; in Python ist `40 + 1.0` bereits `41.0`.
Ohne den Typ-Erhalt in [`wert_aendern`] bräche der Bau — aber aus dem falschen
Grund: nicht weil der Wert ungedeckt ist, sondern weil die Mutation ihn in
einen anderen Typ verwandelt hat. Das Ergebnis wäre ein `BAU` in der Tabelle,
das genau die Verwechslung wiederholt, gegen die dieser Modus gebaut ist.

Gezeigt hat es `build.rs`, nicht die Gegenprobe. Der Typ-Erhalt ist damit eine
**Bedingung dafür, dass die Messung überhaupt misst** — keine Politur.

═══════════════════════════════════════════════════════════════════════════════
 WAS EIN ALTES "gedeckt" BEI EINEM PFLICHTFELD HEISST
═══════════════════════════════════════════════════════════════════════════════

Die Tabelle vom 2026-08-13 führt `spender.damagePerPoint`,
`spender.bonusThreshold` und `spender.bonusMultiplier` als **gedeckt** —
obwohl `uint()` beim Streichen panicken muss. Aus den Rohdaten jenes Laufs
(`/work/tmp/e1-mutation-namen.json`):

    spender.bonusThreshold     61 rote Namen, davon rustartig  0
    spender.damagePerPoint    100 rote Namen, davon rustartig  0
    spender.bonusMultiplier    78 rote Namen, davon rustartig  0
    Kontrolle clearsDoom       13 rote Namen, davon rustartig  6

Die Kontrollzeile ist der Teil, der das trägt: Das Suchmuster findet
Rust-Namen sehr wohl, die drei Nullen sind echt.

Was damals geschah, in vier Schritten:

  1. Der Rust-Bau **brach** (`uint()` panickt beim fehlenden Feld).
  2. Die TypeScript-Suite lief trotzdem — sie braucht kein `cargo`.
  3. Sie färbte, also stand dort `ROT`, und daraus wurde „gedeckt".
  4. Als `BAU` erkannt wurde es nicht: Die Panic-Erkennung kam erst am
     2026-08-21 (siehe den Kommentar bei `rc_bau != 0` weiter unten).

**Das Urteil ist nicht falsch und nicht vollständig.** `vitest` prüft diese
Werte wirklich. Über die **Rust-Seite** sagt der Lauf vom 13.08. bei diesen
Feldern gar nichts, weil sie nie gebaut wurde.

Ein „gedeckt" bei einem Pflichtfeld heißt in der alten Tabelle also: *gedeckt
auf einer Seite, und die andere wurde nicht gemessen.* Wer sie liest, muss das
mitlesen — und `--delta` ist die einzige Mutation, mit der sich die zweite
Seite überhaupt prüfen lässt.
"""

import argparse
import collections
import hashlib
import json
import os
import pathlib
import re
import signal
import shlex
import subprocess
import sys
import time
import threading
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager

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


def raster_ms():
    """Das Takt-Raster des Spiels, aus BEIDEN Quellen gelesen statt geschrieben.

    Ein hier hineingeschriebenes `10` wäre eine dritte Quelle, die still von
    den anderen abdriftet — und dieses Werkzeug benutzt die Zahl, um über
    Messungen zu urteilen. Deshalb wird sie gelesen, und weil es zwei Quellen
    gibt, werden beide gelesen und gegeneinander gehalten:

        src/core/loop.ts   TICK_MS      die Spielseite
        sim/build.rs       RASTER_MS    die Simulatorseite

    Gehen sie auseinander, ist das kein Fall für eine Vorgabe, sondern ein
    Befund über den Bestand — dann bricht der Aufruf ab, statt eine der beiden
    Zahlen zu bevorzugen.
    """
    ts = re.search(
        r"\bTICK_MS\s*(?::\s*number\s*)?=\s*(\d+)",
        (WURZEL / "src/core/loop.ts").read_text(),
    )
    rs = re.search(
        r"\bconst\s+RASTER_MS\s*:\s*u64\s*=\s*(\d+)",
        (WURZEL / "sim/build.rs").read_text(),
    )
    if ts is None or rs is None:
        fehlt = "src/core/loop.ts:TICK_MS" if ts is None else "sim/build.rs:RASTER_MS"
        raise SystemExit(f"Raster nicht lesbar: {fehlt} nicht gefunden. Umbenannt?")
    if int(ts.group(1)) != int(rs.group(1)):
        raise SystemExit(
            f"Raster geht auseinander: loop.ts TICK_MS={ts.group(1)}, "
            f"build.rs RASTER_MS={rs.group(1)}. Das ist ein Befund, keine Vorgabe."
        )
    return int(ts.group(1))


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


def pfade_aller_schulen():
    """Die sechs Zauberdateien, an einer Stelle statt an dreien (#1092)."""
    return {s: WURZEL / f"data/spells/{s}.json" for s in SCHULEN}


def eingecheckter_stand():
    """Der Inhalt der Zauberdateien laut git, nicht laut Arbeitsbaum.

    **Der Bezugspunkt darf nicht aus dem Baum kommen, den er prüfen soll.**
    Läse die Prüfung den aktuellen Inhalt als Sollwert, wäre sie tautologisch
    grün — dieselbe Klasse wie eine Musterliste, die sich selbst befragt.
    """
    stand = {}
    for schule in SCHULEN:
        fertig = subprocess.run(
            ["git", "show", f"HEAD:data/spells/{schule}.json"],
            cwd=WURZEL, capture_output=True, text=True, check=False)
        if fertig.returncode != 0:
            return None            # kein git, kein Bezugspunkt — dann nicht prüfen
        stand[schule] = fertig.stdout
    return stand


def wert_aendern(obj, teile, delta):
    """Ändert eine Zahl um `delta`; gibt (geändert, alt, neu) zurück.

    **Für eine Zahl ist die Streichung die falsche Mutation** (#1087). Jedes
    Feld, das `build.rs` mit `uint()` liest, bricht beim Entfernen IMMER den
    Bau — unabhängig von jedem Prüffall. „BAU" sagt dann nichts über die
    Deckung des WERTS, nur dass das Feld Pflicht ist.

        Streichung      beantwortet „ist das Feld Pflicht"
        Wertänderung    beantwortet „ist der Wert gedeckt"

    Wer das verwechselt, liest die Tabelle falsch — und genau deshalb standen
    dort 33 Felder als gedeckt, die es nicht waren.

    Gibt `(False, None, None)` zurück, wenn der Pfad fehlt oder der Wert keine
    Zahl ist. `bool` ist in Python eine Zahl und wird ausdrücklich
    ausgenommen: Ein `True + 1` wäre `2` und damit weder das eine noch das
    andere.
    """
    for t in teile[:-1]:
        if not isinstance(obj, dict) or t not in obj:
            return (False, None, None)
        obj = obj[t]
    if not isinstance(obj, dict):
        return (False, None, None)
    alt = obj.get(teile[-1], KeyError)
    if alt is KeyError or isinstance(alt, bool) or not isinstance(alt, (int, float)):
        return (False, None, None)
    neu = alt + delta
    # **Ganzzahlig bleiben, wo es ganzzahlig war.** `build.rs:274` lehnt
    # Fließkomma ausdrücklich ab („muss eine ganze Zahl >= 0 sein"), und in
    # Python ist `40 + 1.0` bereits `41.0`. Ohne diese Zeile bräche der Bau —
    # aber aus dem falschen Grund: nicht weil der Wert ungedeckt ist, sondern
    # weil die Mutation ihn in einen anderen Typ verwandelt hat.
    #
    # Das wäre ein `BAU` in der Tabelle gewesen, das genau die Verwechslung
    # wiederholt, gegen die dieser Modus gebaut ist.
    if isinstance(alt, int) and float(neu).is_integer():
        neu = int(neu)
    obj[teile[-1]] = neu
    return (True, alt, neu)


def rote_namen(ausgabe):
    """ALLE Namen der gefärbten Fälle, aus beiden Suiten-Formaten.

    Vollständig, nicht gekappt: Die Klassifikation "hat nur die Datenfrische
    gefärbt oder auch eine Wirkung?" entsteht erst aus dem Vergleich ÜBER alle
    58 Läufe. Ein Test, der bei jeder Streichung färbt, unterscheidet nichts —
    einer, der bei vieren färbt, benennt genau diese vier. Diese Auswertung ist
    gemessen; eine Liste "das sind die Frische-Tests" wäre geraten.
    """
    faelle, dateien = [], []
    for zeile in ausgabe.splitlines():
        z = zeile.strip()
        if z.startswith("test ") and z.endswith("FAILED"):  # cargo
            faelle.append(z[5:-11].strip())
        elif z.startswith("FAIL "):  # vitest: DATEI, nicht Fall
            # `FAIL tests/unit/x.test.ts > describe > it` — vor dem ersten `>`
            # steht der Pfad. Bis zum 2026-08-22 landete er in derselben Liste
            # wie die Fallnamen und wurde mitgezählt: Der Lauf über
            # `decayWard.durationMs` meldete „3 Namen" für zwei gefärbte Fälle.
            dateien.append(z[5:].split(">")[0].strip())
        elif z.startswith("×"):  # vitest: der Fallname
            faelle.append(_ohne_laufzeit(z[1:].split(">")[0].strip()))
    return (list(dict.fromkeys(faelle)), list(dict.fromkeys(dateien)))


def _ohne_laufzeit(name):
    """Die angehängte Laufzeit weg — sonst ist derselbe Fall zweimal derselbe nicht.

    vitest schreibt `× der Schlag liegt gerade noch darin 1ms`. Die Zahl
    schwankt von Lauf zu Lauf, und die Auswertung dieses Werkzeugs vergleicht
    Namen ÜBER Läufe hinweg: „färbt dieser Fall bei vier Feldern oder bei
    allen?" Mit anhängender Laufzeit ist jeder Name einmalig, `dict.fromkeys`
    dedupliziert nichts mehr, und ein Abgleich gegen den Bestand findet den
    Namen nicht wieder.

    **Gemessen, bevor abgeschnitten wird:** Von 2510 `it()`-Namen im Bestand
    endet **keiner** selbst auf ` <Zahl>ms`. Das Muster kann also keinen echten
    Namen verstümmeln — anders als der `lstrip`, den es ersetzt.
    """
    return re.sub(r"\s+\d+ms$", "", name)


def bau_gebrochen(code, ausgabe):
    """Exit-Code zuerst: build.rs-Panics treffen die Compilertextmuster nicht.

    #1055: Ohne den Code wurde ein echter Bauabbruch als ROT verbucht; nach
    getrenntem Bauen/Fahren könnten sogar alte Testbinaries gefahren werden.
    Ein BAU ist kein Nachweis über die Wirkung oder Deckung eines Feldwerts.
    """
    return code != 0 or "error: could not compile" in ausgabe or "error[E" in ausgabe


def suite_nachweis(code, ausgabe, sprache):
    """Nur ausgeführte Fälle ergeben PASS/FAIL; Abbrüche und leere Filter ERROR."""
    text = re.sub(r"\x1b\[[0-9;]*m", "", ausgabe)
    passed = failed = 0
    vollstaendig = False
    if sprache == "rust":
        summen = re.findall(r"^test result: \w+\. (\d+) passed; (\d+) failed", text, re.M)
        if summen:
            passed = sum(int(p) for p, _ in summen)
            failed = sum(int(f) for _, f in summen)
            starts = re.findall(r"^running \d+ tests?\s*$", text, re.M)
            targets = re.findall(r"^\s*Running .+ \([^\n]+\)\s*$", text, re.M)
            # Cargo kann nach normalen roten Fällen einen weiteren Testprozess
            # durch ein Signal verlieren. Vorige Summen belegen dann NICHT den
            # gesamten Lauf. Nulltests sind bei bin/example-Zielen regulär;
            # auch sie müssen ihre eigene Abschlusszeile liefern.
            abbruch = re.search(
                r"signal:|SIGSEGV|SIGABRT|segmentation fault|"
                r"process didn't exit successfully|could not execute process|"
                r"failed to (?:spawn|execute|run)|fatal runtime error|test process aborted|"
                r"error: could not compile|error\[E", text, re.I)
            vollstaendig = (not starts or len(starts) == len(summen)) \
                and (not targets or len(targets) == len(summen)) and not abbruch
        else:
            p = re.search(r"Tests bestanden:\s*(\d+)", text)
            s = re.search(r"Suiten ausgefuehrt:\s*(\d+) von (\d+)", text)
            stumm = re.search(r"Ohne Ergebniszeile:\s*(\d+)", text)
            fehlen = re.search(r"Binaries fehlten:\s*(\d+)", text)
            failed = len(re.findall(r"^\s*test .+ \.\.\. FAILED\s*$", text, re.M))
            passed = int(p.group(1)) if p else 0
            vollstaendig = bool(p and s and stumm and fehlen
                               and int(s.group(1)) == int(s.group(2)) > 0
                               and int(stumm.group(1)) == int(fehlen.group(1)) == 0)
    elif sprache == "ts":
        zeilen = re.findall(r"^\s*Tests\s+(.+)$", text, re.M)
        if len(zeilen) == 1:
            werte = dict((k, int(n)) for n, k in re.findall(r"(\d+) (passed|failed)", zeilen[0]))
            passed, failed = werte.get("passed", 0), werte.get("failed", 0)
            vollstaendig = bool(werte)
    else:
        raise ValueError(f"Unbekannte Suite: {sprache}")
    stand = "ERROR"
    if vollstaendig and passed + failed > 0:
        if code == 0 and failed == 0:
            stand = "PASS"
        elif code != 0 and failed > 0:
            stand = "FAIL"
    return {"exit": code, "status": stand, "passed": passed, "failed": failed,
            "executed": passed + failed, "complete_summary": vollstaendig}


def alte_urteile(pfad):
    """Liest das historische Urteil, ohne 'gedeckt' zu einem Rust-PASS umzudeuten."""
    urteile = {}
    for zeile in pfad.read_text().splitlines():
        spalten = [s.strip() for s in zeile.strip().strip("|").split("|")]
        if len(spalten) >= 5 and re.fullmatch(r"`[^`]+`", spalten[0]):
            urteile[spalten[0].strip("`")] = spalten[3]
    if not urteile:
        raise ValueError(f"Keine historischen Feldurteile gefunden: {pfad}")
    return urteile


def bau_vergleich(alt, nachweis):
    """Benannte Änderungen; ein Teil- oder Nur-Bau-Lauf ist kein Gesamtabschluss."""
    gemessen = nachweis["fields"]
    neu_bau = [{"field": feld, "old_verdict": urteil,
                "build_exit": gemessen[feld]["build"]["exit"]}
               for feld, urteil in sorted(alt.items())
               if gemessen.get(feld, {}).get("status") == "BAU"]
    ungemessen = sorted(set(alt) - {
                f for f, eintrag in gemessen.items()
                if eintrag.get("status") in ("BAU", "ROT", "GRUEN")})
    return {"historical_fields": len(alt), "new_bau": neu_bau,
            "unmeasured_old_fields": ungemessen,
            "new_fields": sorted(set(nachweis["all_fields"]) - set(alt)),
            "full_pass": nachweis.get("full_pass", False) and not ungemessen}


def schreibe(ziel, ergebnisse):
    zeilen = ["| Feld | Zauber | Ergebnis | Detail |", "| --- | ---: | --- | --- |"]
    zeilen += [f"| `{f}` | {n} | {s} | {d} |" for f, s, n, d in ergebnisse]
    # Die Fußnote steht in der TABELLE, nicht nur im Quelltext dieses Werkzeugs:
    # Gelesen wird die Tabelle, und die Zahl in der Detail-Spalte lädt zur
    # falschen Deutung ein, solange nichts danebensteht.
    zeilen += [
        "",
        f"{len(ergebnisse)} Felder gefahren.",
        "",
        "Die Zahl in *Detail* zählt **Namen**, nicht gefärbte Fälle: Die Auswertung",
        "dedupliziert (`dict.fromkeys`), und ein Testname wie",
        "`die_pruefsumme_stimmt_in_jedem_takt` steht in 38 Dateien und erscheint",
        "als ein Eintrag. Die Zahl ist eine Untergrenze.",
        "",
        "**BAU** heißt: Der Rust-Bau brach ab; die Rust-Suite wurde nicht gestartet.",
        "Die TS-Suite kann trotzdem gelaufen sein. Der JSON-Nachweis hält Bau,",
        "Rust und TS getrennt. BAU belegt weder eine Wertwirkung noch Rust-Testdeckung.",
        "**FEHLER** heißt: Eine Suite brach ab oder führte keine Tests aus; das ist kein ROT.",
        "**baut** stammt ausschließlich aus --nur-bau und enthält keinen Testnachweis.",
        "",
        "**GRUEN?** heißt: Der Schritt lag neben dem 10-ms-Raster, die Änderung kann",
        "eine gerasterte Größe also gar nicht erreicht haben. Weder gedeckt noch",
        "ungedeckt — *nicht gemessen*.",
        "",
        "**Und ein sauberes GRUEN belegt „ungedeckt“ nur mit der Gegenrichtung.**",
        "Gemessen an `decayWard.durationMs`: `+10` bleibt grün (das Fenster wandert",
        "von 12000 auf 12010, der Prüffall bei 12000 bleibt drinnen), `-10` färbt.",
        "Ein einzelner Lauf trennt „das Feld ist ungedeckt“ nicht von „diese",
        "Richtung ist wirkungslos“.",
    ]
    ziel.write_text("\n".join(zeilen) + "\n")


# Eigenes Bauprotokoll: trennt build.rs-Abbrüche vom anschließenden Testlauf.
BAU_LOG = "/work/tmp/e1-1045-bau.log"


# TS darf parallel zum seriellen Rust-Zweig laufen. Beide eigenen Gruppen
# müssen vor jedem Rückbau beendet und vom Executor abgewartet werden.
_LAUFEND = set()
_LAUFEND_LOCK = threading.RLock()
_ABBRUCH = threading.Event()
_SIGTERM = False


def _unterbrechung_pruefen():
    if _SIGTERM:
        raise SystemExit(143)
    if _ABBRUCH.is_set():
        raise InterruptedError("Ein anderer Prüfzweig wurde abgebrochen")


def lauf(befehl, timeout):
    # `start_new_session=True` gibt dem KIND eine eigene Prozessgruppe — nicht
    # diesem Prozess. Das ist der Unterschied, auf den es ankommt:
    #
    #   os.setsid() hier      der ganze Durchgang verlässt seine Sitzung und
    #                         verhält sich bei Terminal-Signalen anders
    #   hier am Kind          nur `npm`/`cargo` und deren Enkel bilden eine
    #                         Gruppe; dieser Prozess bleibt, wo er war
    #
    # Und es macht AC3 strukturell wahr statt nur wahrscheinlich: `killpg` auf
    # die Gruppe des eigenen Kindes KANN keinen fremden Prozess treffen. Ein
    # Muster-basiertes Aufräumen (`pkill node`) könnte es, und auf dieser
    # Maschine laufen sechs Claude-Sitzungen und der Browser des Auftraggebers.
    p = None
    try:
        with _LAUFEND_LOCK:
            _unterbrechung_pruefen()
            p = subprocess.Popen(befehl, shell=True, cwd=WURZEL, text=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 start_new_session=True)
            _LAUFEND.add(p)
        # SIGTERM kann während Popen eintreffen, bevor es eine PID im Register
        # gibt. Der Handler wirft nicht; erst nach der Registrierung aufräumen.
        _unterbrechung_pruefen()
        aus, _ = p.communicate(timeout=timeout)
        _unterbrechung_pruefen()
    except subprocess.TimeoutExpired:
        # Auch der Timeout muss die GRUPPE treffen. Am 2026-08-22 überlebten
        # vier vitest-Worker mit je 99 % CPU den Tod ihres Vaters und rechneten
        # in einer Endlosschleife weiter.
        _alle_beenden()
        raise
    except BaseException:
        _alle_beenden()
        raise
    finally:
        if p is not None and p.stdout is not None:
            p.stdout.close()
        with _LAUFEND_LOCK:
            _LAUFEND.discard(p)
    return p.returncode, (aus or "")


def _gruppe_beenden(p):
    """Beendet die Prozessgruppe eines Kindes, erst freundlich, dann hart."""
    if p is None:
        return
    for signal_nr, gnade in ((signal.SIGTERM, 5), (signal.SIGKILL, 0)):
        try:
            # start_new_session setzt PGID=PID. Die Gruppe kann noch leben,
            # nachdem die Shell endet; deshalb nicht vorher p.poll() prüfen.
            os.killpg(p.pid, signal_nr)
        except ProcessLookupError:
            break
        if gnade:
            try:
                p.wait(timeout=gnade)
            except subprocess.TimeoutExpired:
                pass
    p.wait()


def _alle_beenden():
    with _LAUFEND_LOCK:
        _ABBRUCH.set()  # Kein spät startender Thread darf noch ein Kind erzeugen.
        kinder = list(_LAUFEND)
    for kind in kinder:
        _gruppe_beenden(kind)


@contextmanager
def ts_nebenbei(arbeit):
    """Ein TS-Zweig; auch bei Fehler/SIGTERM erst joinen, dann Daten rückbauen."""
    def geschuetzt():
        try:
            return arbeit()
        except BaseException:
            _alle_beenden()
            raise

    with ThreadPoolExecutor(max_workers=1) as pool:
        zukunft = pool.submit(geschuetzt)
        try:
            yield zukunft
            zukunft.result()
            _unterbrechung_pruefen()
        except BaseException as error:
            zukunft.cancel()
            _alle_beenden()
            if isinstance(error, InterruptedError) and not zukunft.cancelled():
                zukunft.result()  # Den ursprünglichen TS-Fehler erhalten.
            raise


def main():
    # SIGTERM MUSS EINEN AUSGANG NEHMEN, der `finally` erreicht. Ohne diesen
    # Handler beendet das Signal den Prozess hart, die Sicherungskopie wird nie
    # zurückgeschrieben — und der mutierte Datensatz bleibt im Arbeitsbaum
    # liegen. Genau so passiert beim ersten Lauf dieses Skripts: Der
    # Bash-Timeout griff nach zehn Minuten, und `searing` stand danach ohne
    # `dot.tickDamage` im Baum. Jeder folgende Lauf hätte gegen kaputte Daten
    # gemessen, ohne dass etwas darauf hingewiesen hätte.
    #
    # UND ER MUSS DIE KINDER MITNEHMEN. `sys.exit()` allein beendet nur diesen
    # Prozess; die Enkel laufen weiter, und zwar gerade dann, wenn es weh tut:
    # Ein Abbruch passiert, wenn eine Messung schiefläuft — also wenn die
    # Maschine ohnehin unter Druck steht. Am 2026-08-22 rechneten nach einem
    # SIGTERM vier verwaiste vitest-Worker mit je 99 % CPU weiter, in der
    # Endlosschleife des gerade gefahrenen Feldes. Sie mussten von Hand gesucht
    # werden, und die nächste Messung wäre gegen eine Maschine unter Volllast
    # gelaufen.
    def _abbruch(*_):
        global _SIGTERM
        _SIGTERM = True
        # Niemals wait()/Locks im Handler: communicate kann gerade den
        # internen, nicht-reentranten Popen-Waitlock halten. Nur signalisieren;
        # lauf räumt nach dem Unwinden auf und wartet die Kinder dort ab.
        for kind in list(_LAUFEND):
            try:
                os.killpg(kind.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    signal.signal(signal.SIGTERM, _abbruch)

    ap = argparse.ArgumentParser()
    ap.add_argument("--feld", help="nur dieses Feld (Trägerprobe)")
    ap.add_argument("--ab", help="erst ab diesem Feld beginnen")
    ap.add_argument("--aus", default="docs/streich-mutation-ergebnis.md")
    ap.add_argument("--roh", default="/work/tmp/e1-mutation-namen.json",
                    help="alle roten Testnamen je Feld, Grundlage der Auswertung")
    ap.add_argument("--nachweis", help="vollständiger JSON-Nachweis; sonst --aus mit Endung .json")
    ap.add_argument("--vergleich", help="historischer Bericht für den namentlichen BAU-Abgleich")
    ap.add_argument("--bau-log", default=BAU_LOG, help="Bauprotokoll dieses isolierten Laufs")
    ap.add_argument("--timeout", type=int, default=1800)
    ap.add_argument("--delta", type=float, default=None, metavar="N",
                    help="WERTAENDERUNG statt Streichung: addiert N auf den "
                         "Zahlenwert. Beantwortet 'ist der WERT gedeckt' statt "
                         "'ist das Feld Pflicht' (#1087). Fuer Felder, die "
                         "build.rs mit uint() liest, ist das die einzige "
                         "Mutation, die den Bau nicht ohnehin bricht.")
    ap.add_argument("--nur-bau", action="store_true",
                    help="nur bauen, nicht fahren — beantwortet 'bricht die "
                         "Streichung den Bau?' in rund 45 s statt 4 min je Feld")
    args = ap.parse_args()

    # NUR bei `--delta` gelesen: Ohne Wertänderung gibt es keine Schrittweite,
    # gegen die etwas zu prüfen wäre — und ein Abbruch wegen umbenannter
    # Konstante träfe dann einen Lauf, der die Zahl gar nicht braucht.
    RASTER = raster_ms() if args.delta is not None else None

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
    # Zwei serialisierte REMOTE-Schritte gemäß aktueller Rechnerregel: zuerst
    # nur bauen, dann dort testen. Keine lokalen Rust-Binaries starten, keine
    # Artefakte und insbesondere keine Cargo.lock vom Buildserver kopieren.
    # Cargo gibt standardmäßig alle Fallnamen aus; --no-fail-fast verhindert,
    # dass ein rotes Target die Prüfung der übrigen Targets abschneidet.
    rust_bau = "cargo remote --no-copy-lock -- test --no-run --all-targets"
    bau_log = WURZEL / args.bau_log
    rust_lauf = "cargo remote --no-copy-lock -- test --all-targets --no-fail-fast"
    ts = "npx --no-install vitest run"

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
    if not felder:
        print("FEHLER: keine Wirkfelder gefunden — ein leerer Durchgang ist keine Messung.")
        return 2

    ziel = WURZEL / args.aus
    nachweis_pfad = WURZEL / args.nachweis if args.nachweis else ziel.with_suffix(".json")
    protokolle = nachweis_pfad.with_name(nachweis_pfad.stem + "-logs")
    protokolle.mkdir(parents=True, exist_ok=True)
    alter_stand = alte_urteile(WURZEL / args.vergleich) if args.vergleich else None
    nachweis = {"schema": 1, "mode": "delta" if args.delta is not None else "delete",
                "delta": args.delta, "build_only": args.nur_bau,
                "all_fields": sorted(wo), "requested_fields": felder,
                "completed": False, "full_pass": False, "baseline": {}, "fields": {},
                "source_sha256": {str(p.relative_to(WURZEL)): hashlib.sha256(p.read_bytes()).hexdigest()
                                  for p in pfade_aller_schulen().values()}}
    for schluessel, argumente in (
            ("git_head", ["git", "rev-parse", "HEAD"]),
            ("source_status", ["git", "status", "--porcelain", "--", "data", "src", "sim", "tests", "tools"])):
        bezug = subprocess.run(argumente, cwd=WURZEL, capture_output=True, text=True, check=False)
        nachweis[schluessel] = bezug.stdout.strip() if bezug.returncode == 0 else None
    nachweis["tool_sha256"] = hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()

    def speichern():
        if alter_stand is not None:
            nachweis["historical_comparison"] = bau_vergleich(alter_stand, nachweis)
        nachweis_pfad.parent.mkdir(parents=True, exist_ok=True)
        temporaer = nachweis_pfad.with_suffix(nachweis_pfad.suffix + ".tmp")
        temporaer.write_text(json.dumps(nachweis, indent=2, ensure_ascii=False) + "\n")
        temporaer.replace(nachweis_pfad)

    def stufe(probe, name, befehl):
        log = protokolle / f"{probe}.{name}.log"
        begonnen, uhr = time.time(), time.monotonic()
        try:
            code, ausgabe = lauf(befehl, args.timeout)
        except subprocess.TimeoutExpired:
            log.write_text("TIMEOUT: Prozessgruppe beendet; diese Stufe ist nicht abgeschlossen.\n")
            raise
        if name == "build" and bau_log.exists():
            ausgabe += bau_log.read_text()
        log.write_text(ausgabe)
        eintrag = ({"exit": code, "status": "FAIL" if bau_gebrochen(code, ausgabe) else "PASS"}
                   if name == "build" else suite_nachweis(code, ausgabe, name))
        eintrag.update(log=str(log), started_unix=begonnen, seconds=time.monotonic() - uhr)
        return code, ausgabe, eintrag

    def bauen(probe):
        return stufe(probe, "build", f"{rust_bau} > {shlex.quote(str(bau_log))} 2>&1")

    def messen(probe, eintrag):
        # Der Datensatz bleibt bis zum Ende BEIDER Zweige unverändert.
        # Nur TS überlappt; Remote-Bau und Remote-Tests bleiben seriell.
        with ts_nebenbei(lambda: stufe(probe, "ts", ts)) as ts_ergebnis:
            rc_bau, aus_bau, eintrag["build"] = bauen(probe)
            if bau_gebrochen(rc_bau, aus_bau):
                rc_r, aus_r = rc_bau, aus_bau
                eintrag["rust"] = {"status": "SKIPPED", "reason": "build-failure", "executed": 0}
            else:
                rc_r, aus_r, eintrag["rust"] = stufe(probe, "rust", rust_lauf)
            rc_t, aus_t, eintrag["ts"] = ts_ergebnis.result()
        return rc_bau, aus_bau, rc_r, aus_r, rc_t, aus_t

    speichern()

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
    # ═══════════════════════════════════════════════════════════════════════
    #  IST DER DATENSATZ ÜBERHAUPT SAUBER, BEVOR WIR ANFANGEN? (#1092)
    # ═══════════════════════════════════════════════════════════════════════
    #
    # Der Läufer stellt am Ende zurück — im `finally` und über einen
    # SIGTERM-Handler. **Bei SIGKILL läuft beides nicht.** Am 2026-08-22 hat
    # ein harter Abbruch einen Lauf mitten in der Nullprobe beendet; er starb
    # zufällig VOR dem Setzen der Mutation, und der Datensatz blieb sauber.
    # Dreißig Sekunden später hätte die Mutation dringestanden.
    #
    # Wahrscheinlichste Ursache war der OOM-Killer: `earlyoom` läuft hier mit
    # `-m 10`, der Speicher lag bei 6,6 Prozent, und weder `python3` noch
    # `cargo` noch `node24` stehen auf seiner Schutzliste — `claude` schon.
    # Der Schutz deckt die Sitzungen, nicht ihre Arbeit.
    #
    # **Ein Aufräumversprechen, das bei SIGKILL nicht läuft, ist die falsche
    # Stelle.** Die billigere ist hier: Der NÄCHSTE Lauf merkt, dass der Baum
    # verunreinigt ist, bevor er gegen ihn misst.
    #
    # Die Nullprobe allein fängt es NICHT: Sie färbt nur, wenn ein Prüffall
    # den Fremdwert bemerkt. Bei einem Feld ohne spezifischen Prüffall bleibt
    # sie grün — und dann misst der ganze Lauf gegen eine stille Mutation.
    urfassung = eingecheckter_stand()
    verschmutzt = [] if urfassung is None else [
        s for s, p in pfade_aller_schulen().items()
        if p.read_text() != urfassung[s]
    ]
    if urfassung is None:
        print("HINWEIS: git nicht erreichbar — der Vorher-Abgleich entfaellt.",
              file=sys.stderr)
        print("         Ein liegengebliebener Fremdwert faellt dann NICHT auf.",
              file=sys.stderr)
    if verschmutzt:
        print("ABGELEHNT: der Datensatz weicht vom eingecheckten Stand ab —",
              file=sys.stderr)
        for s in verschmutzt:
            print(f"           data/spells/{s}.json", file=sys.stderr)
        print("           Wahrscheinlich hat ein harter Abbruch eine Mutation",
              file=sys.stderr)
        print("           liegen lassen. `git checkout -- data/spells/` und",
              file=sys.stderr)
        print("           erneut starten. NICHT gemessen wird gegen einen Baum,",
              file=sys.stderr)
        print("           von dem niemand weiss, was in ihm steht.", file=sys.stderr)
        return 2

    print("Nullprobe (ohne Mutation) ...", flush=True)
    # AUCH HIER ZWEI SCHRITTE. Die Nullprobe MUSS bauen: Ohne den Bau faehrt sie
    # gegen Binaries aus einem frueheren Lauf und misst einen Baum, der so gar
    # nicht mehr existiert.
    rc_bau, aus_bau, nachweis["baseline"]["build"] = bauen("baseline")
    speichern()
    if bau_gebrochen(rc_bau, aus_bau):
        print(f"ABBRUCH: Der Bau bricht schon vor der ersten Mutation "
              f"(rc={rc_bau}). Siehe {bau_log}.")
        return 3
    if args.nur_bau:
        # Die Suiten bleiben stehen, und die Nullprobe schrumpft entsprechend:
        # Sie kann nur noch zusichern, dass der Bau VOR der ersten Mutation
        # steht — das hat der Aufruf oben bereits getan. Ein roter Baum bliebe
        # hier unentdeckt; er kann das Ergebnis aber auch nicht verfälschen,
        # weil kein Testergebnis in die Klassifikation eingeht.
        print("Nullprobe: Bau steht. Suiten bleiben aus (--nur-bau).\n", flush=True)
    else:
        rc_r, _, nachweis["baseline"]["rust"] = stufe("baseline", "rust", rust_lauf)
        rc_t, _, nachweis["baseline"]["ts"] = stufe("baseline", "ts", ts)
        speichern()
        if any(nachweis["baseline"][sprache]["status"] != "PASS" for sprache in ("rust", "ts")):
            print(f"ABBRUCH: Der Baum ist schon vor der ersten Mutation rot "
                  f"(rust={rc_r}, ts={rc_t}). Jedes Feld sähe gedeckt aus.")
            return 3
        print("Nullprobe gruen — gemessen wird gegen einen sauberen Baum.\n", flush=True)

    # Die Kopfzeile nennt, WAS gefahren wird — im Nur-Bau-Modus ist das etwas
    # anderes, und ein Protokoll, das "je ein Lauf beider Suiten" behauptet,
    # während keine Suite läuft, ist später nicht mehr auseinanderzuhalten.
    print(f"{len(felder)} Felder, "
          f"{'nur Bau, keine Suite' if args.nur_bau else 'je ein Lauf beider Suiten'}\n",
          flush=True)
    ergebnisse = []
    roh = {}
    for i, feld in enumerate(felder, 1):
        _unterbrechung_pruefen()
        teile = feld.split(".")
        eintrag = {"status": "UNGEPRUEFT", "mutated_spells": 0}
        nachweis["fields"][feld] = eintrag
        probe = f"{i:03d}-" + re.sub(r"[^A-Za-z0-9_.-]", "_", feld)
        # Sicherungskopien ALLER Schulen, nicht nur der betroffenen: Ein Feld
        # kann in mehreren stehen (control.cost in fixation, heal.amount in
        # coagulation), und ein halb zurückgesetzter Datensatz verfälscht jeden
        # folgenden Lauf.
        pfade = {s: WURZEL / f"data/spells/{s}.json" for s in SCHULEN}
        sicherung = {s: p.read_bytes() for s, p in pfade.items()}
        getroffen = 0
        # Was die Wertänderung TATSÄCHLICH getan hat, je Zauber. Ohne diese
        # Aufzeichnung sähe „GRÜN" bei einem Aufzählungsfeld genauso aus wie
        # bei einem ungedeckten Wert — man wüsste nicht, ob 0 zu 1 wurde
        # (ein anderer gültiger Fall) oder 15 zu 16 (derselbe Fall, knapp
        # daneben).
        aenderungen = []
        try:
            for schule, p in pfade.items():
                d = json.loads(p.read_text())
                liste = d if isinstance(d, list) else d["spells"]
                geaendert = False
                for s in liste:
                    if args.delta is None:
                        if streichen(s, teile):
                            getroffen += 1
                            geaendert = True
                    else:
                        ok, alt, neu = wert_aendern(s, teile, args.delta)
                        if ok:
                            getroffen += 1
                            geaendert = True
                            aenderungen.append((s.get("id", "?"), alt, neu))
                if geaendert:
                    p.write_text(json.dumps(d, indent=2, ensure_ascii=False) + "\n")

            # ZUSICHERUNG IN DEN DATENSATZ: Wurde überhaupt etwas gestrichen?
            # Ohne diese Zahl sieht ein Tippfehler im Pfad wie ein ungedecktes
            # Feld aus — der Lauf bleibt grün, weil er nichts geändert hat.
            #
            # **Bei `--delta` trägt sie doppelt.** Dort gibt es einen zweiten
            # Weg, auf dem nichts passiert: Das Feld existiert, ist aber keine
            # Zahl (Text, Liste, `true`). `wert_aendern` lässt es dann in Ruhe
            # und meldet `False` — und ohne diesen Abgleich stünde das Feld als
            # GRÜN in der Tabelle, obwohl es nie angefasst wurde.
            if getroffen != len(wo[feld]):
                erwartet = len(wo[feld])
                grund = "geaendert" if args.delta is not None else "gestrichen"
                zusatz = f"erwartet {erwartet}"
                if args.delta is not None and getroffen == 0:
                    zusatz += " — keine ZAHL? Dann ist --delta hier das falsche Werkzeug"
                ergebnisse.append((feld, "FEHLER", getroffen, zusatz))
                eintrag.update(status="FEHLER", mutated_spells=getroffen, detail=zusatz)
                print(f"[{i}/{len(felder)}] {feld:<38} FEHLER: {getroffen} statt {erwartet} {grund}", flush=True)
                continue

            eintrag.update(mutated_spells=getroffen, changes=aenderungen)

            # **Was die Änderung wirklich getan hat, in die Ausgabe.** Ein
            # späteres GRÜN ist ohne diese Zeile nicht deutbar: Man sieht nicht,
            # ob 15 zu 16 wurde (derselbe Fall, knapp daneben) oder 0 zu 1 (bei
            # einer Aufzählung ein völlig anderer Fall). AC3 hängt daran.
            if args.delta is not None and aenderungen:
                proben = ", ".join(f"{sid}: {alt}->{neu}" for sid, alt, neu in aenderungen[:3])
                if len(aenderungen) > 3:
                    proben += f", … ({len(aenderungen)} gesamt)"
                # **Die Schrittweite gehört daneben, nicht nur der neue Wert**
                # (AC6): `6000->6010` allein sagt nicht, ob 10 überhaupt
                # ankommen kann. Erst „Schritt 10, Raster 10 ms" macht die
                # Frage sichtbar, ohne sie zu beantworten.
                schritt = f"Schritt {args.delta:g}"
                if feld.split(".")[-1].endswith("Ms"):
                    schritt += f", Raster {RASTER} ms"
                print(f"      {proben}   [{schritt}]", flush=True)

            t0 = time.monotonic()
            # `rc_bau != 0` STEHT VORNE, und zwar aus einem gemessenen Grund:
            # Ein Panic in `build.rs` bricht den Bau mit Exit 101, schreibt aber
            # WEDER "could not compile" NOCH "error[E" ins Log — dort steht
            # "failed to run custom build command" und "panicked at". Gemessen am
            # 2026-08-21 durch Streichen eines Pflichtfeldes: beide Textmuster
            # fanden nichts, der Exit-Code war 101.
            #
            # Das Textmuster stammt aus dem Bestand und war dort HARMLOS: `cargo
            # test` faehrt bei gebrochenem Bau nichts, `rc_r` wurde ungleich 0,
            # und der Fall landete als ROT statt BAU — falsch einsortiert, aber
            # ohne falsche Zahlen. Beim historischen Umbau auf lokale Binaries
            # (#1045) hätte der Lauf sogar die Namen des VORIGEN Feldes gemeldet.
            # Seit 2026-09-06 laufen beide Schritte wieder vollständig remote.
            #
            # DER BAU-FEHLER ENTSTEHT IM ERSTEN SCHRITT, nicht im zweiten.
            # Wuerde `gebrochen` die LAUF-Ausgabe pruefen, saehe ein Feld,
            # dessen Mutation den Bau bricht, aus wie eines, das nichts bewirkt —
            # also wie totes Gewicht, das man entfernen kann.
            if args.nur_bau:
                rc_bau, aus_bau, eintrag["build"] = bauen(probe)
            else:
                rc_bau, aus_bau, rc_r, aus_r, rc_t, aus_t = messen(probe, eintrag)
            gebrochen = bau_gebrochen(rc_bau, aus_bau)

            # BRICHT DER BAU, WIRD NICHT GEFAHREN. Der Rust-Nachweis bleibt
            # ausdrücklich SKIPPED; TS kann unabhängig davon echte Tests fahren.
            # NUR-BAU: Hier endet der Durchgang für dieses Feld. Der Modus
            # beantwortet EINE Frage — "bricht die Streichung den Bau?" — und
            # zwar in rund 45 s statt vier Minuten.
            #
            # WAS ER NICHT BEANTWORTET, und das gehört an die Stelle, an der
            # jemand ihn benutzt:
            #
            #   GRUEN gegen ROT   entsteht erst aus dem Lauf. "kein BAU" heißt
            #                     NICHT "gedeckt".
            #   HAENGT            Ein Feld, dessen Streichung eine Endlosschleife
            #                     erzeugt, baut sauber. `channelTickMs` gemessen
            #                     am 2026-08-22: vier vitest-Worker, je 99 %,
            #                     über 770 CPU-Sekunden, kein Ende in Sicht.
            #   OOM               Ein an Speichermangel gestorbener Lauf liefert
            #                     rc != 0 und sähe im vollen Weg wie ROT aus.
            #                     Hier fällt er gar nicht erst an.
            if args.nur_bau:
                dauer = time.monotonic() - t0
                stand = "BAU" if gebrochen else "baut"
                detail = "Build bricht ab" if gebrochen else "Bau geht durch"
                eintrag.update(status=stand, detail=detail,
                               rust={"status": "SKIPPED", "reason": "build-only"},
                               ts={"status": "SKIPPED", "reason": "build-only"})
                ergebnisse.append((feld, stand, getroffen, detail))
                print(f"[{i}/{len(felder)}] {feld:<38} {stand:<6} {getroffen:>2} Zauber  "
                      f"{detail}  ({dauer:.0f} s)", flush=True)
                schreibe(WURZEL / args.aus, ergebnisse)
                _unterbrechung_pruefen()
                continue

            dauer = time.monotonic() - t0

            if gebrochen:
                stand, detail = "BAU", "Build bricht ab"
            elif any(eintrag[sprache]["status"] == "ERROR" for sprache in ("rust", "ts")):
                stand, detail = "FEHLER", "Suite unvollständig oder ohne ausgeführte Fälle"
            elif rc_r != 0 or rc_t != 0:
                stand = "ROT"
                # DIE NAMEN, nicht die Zahl. "Eine Prüfung färbt" war auch dann
                # wahr, als die Ursache ein fremder Boss im Arbeitsbaum war —
                # Zahl, Exit-Code und Satz stimmten alle. Erst der Name
                # `parity_boss_state` zeigte, dass die Streichung damit nichts
                # zu tun hatte.
                f_r, d_r = rote_namen(aus_r)
                f_t, d_t = rote_namen(aus_t)
                namen = list(dict.fromkeys(f_r + f_t))
                dateien = list(dict.fromkeys(d_r + d_t))
                # **DIE HERKUNFT WIRD MITGESCHRIEBEN, nicht später erraten.**
                # Aus der alten flachen Liste ließ sich nur noch nach der FORM
                # schließen, welcher Name aus welcher Suite kam
                # (`snake_case_ohne_leerzeichen` = vermutlich cargo). Das ist
                # eine Vermutung über eine Schreibweise, und genau diese Klasse
                # hat am 2026-08-21 einen Rust-Befund getragen, dessen 185 rote
                # Fälle sämtlich aus der TS-Suite kamen.
                #
                # Hier liegen beide Listen ohnehin getrennt vor. Sie
                # zusammenzuwerfen und die Trennung hinterher zu rekonstruieren
                # wäre Arbeit, um Information wegzuwerfen.
                roh[feld] = {
                    "faelle": namen,
                    "dateien": dateien,
                    "rust": f_r,
                    "ts": f_t,
                }
                (WURZEL / args.roh).write_text(json.dumps(roh, indent=1, ensure_ascii=False))
                # "NAMEN", nicht "Faelle" — und das ist keine Wortklauberei:
                # `rote_namen()` endet auf `dict.fromkeys`, dedupliziert also.
                # `die_pruefsumme_stimmt_in_jedem_takt` steht in 38 Testdateien
                # und erscheint hier als EIN Eintrag. Die Zahl ist damit eine
                # Untergrenze der gefärbten Fälle, keine Fallzahl.
                #
                # Bis zum 2026-08-22 stand "Faelle" da. Wer die Spalte las, hielt
                # eine 8 für acht gefärbte Prüfungen; es können achtzig gewesen
                # sein, verteilt auf acht Namen.
                # **Fälle und Dateien getrennt**, weil sie verschiedene Dinge
                # sind: `2 Namen in 1 Datei` sagt etwas, `3 Namen` sagte nichts
                # und war obendrein falsch.
                if namen:
                    detail = f"{len(namen)} Namen"
                    if dateien:
                        detail += f" in {len(dateien)} Datei" + ("en" if len(dateien) > 1 else "")
                elif dateien:
                    # Kann vorkommen, wenn eine Suite abbricht, bevor ein
                    # einzelner Fall gedruckt wird — die Datei ist rot, ein
                    # Fallname existiert nicht. Das ist NICHT „kein Fall
                    # gefärbt", sondern „kein Fall BENANNT".
                    detail = f"rot in {len(dateien)} Datei(en), kein Fall benannt"
                else:
                    detail = "rot ohne benannten Fall"
            elif args.delta is not None and feld.split(".")[-1].endswith("Ms") \
                    and abs(args.delta) % RASTER != 0:
                # AC6 von #1087: Ein Schritt, der NEBEN dem Raster liegt, kann
                # eine gerasterte Größe gar nicht erreichen — 12000 und 12001
                # laufen beide erst bei 12010 ab. Das Ergebnis sieht aus wie
                # "keine Prüfung sieht den Wert" und heißt "die Änderung ist
                # nie angekommen".
                #
                # DIE PRÜFUNG IST MECHANISCH, nicht bedeutungsraten: Endung
                # `Ms` und Schrittweite gegen `RASTER`. Der Versuch, Felder
                # nach ihren WERTEN einzuteilen, hat sechs von 45 falsch
                # sortiert (siehe Dateikopf) — diese hier fragt nur, ob der
                # SCHRITT auf dem Raster liegt, und das steht fest.
                stand = "GRUEN?"
                detail = f"Schritt {args.delta:g} liegt neben dem Raster {RASTER} ms"
            else:
                stand, detail = "GRUEN", "beide Suiten gruen"
            eintrag.update(status=stand, detail=detail, seconds=dauer,
                           failure_names={"rust": [] if gebrochen else rote_namen(aus_r)[0],
                                          "ts": rote_namen(aus_t)[0]})
            ergebnisse.append((feld, stand, getroffen, detail))
            print(f"[{i}/{len(felder)}] {feld:<38} {stand:<6} {getroffen:>2} Zauber  "
                  f"{detail}  ({dauer/60:.1f} min)", flush=True)
            # NACH JEDEM FELD schreiben, nicht erst am Ende: Ein Lauf über 62
            # Felder dauert Stunden, und ein Abbruch in Stunde drei darf nicht
            # alles Gemessene verwerfen.
            schreibe(WURZEL / args.aus, ergebnisse)
            _unterbrechung_pruefen()
        except BaseException as error:
            eintrag.update(status="ABBRUCH", detail=f"{type(error).__name__}: {error}")
            raise
        finally:
            for schule, p in pfade.items():
                p.write_bytes(sicherung[schule])
            eintrag["restored"] = all(p.read_bytes() == sicherung[s] for s, p in pfade.items())
            speichern()

    _unterbrechung_pruefen()
    ziel = WURZEL / args.aus
    schreibe(ziel, ergebnisse)
    _unterbrechung_pruefen()
    nachweis["completed"] = len(ergebnisse) == len(felder)
    fehler = any(e.get("status") in ("FEHLER", "ABBRUCH")
                 or any(e.get(sprache, {}).get("status") == "ERROR" for sprache in ("rust", "ts"))
                 for e in nachweis["fields"].values())
    nachweis["full_pass"] = (nachweis["completed"] and not fehler and not args.nur_bau
                              and args.delta is None and not args.feld and not args.ab)
    speichern()
    print(f"\n{ziel}")
    gruen = [f for f, s, _, _ in ergebnisse if s == "GRUEN"]
    fraglich = [f for f, s, _, _ in ergebnisse if s == "GRUEN?"]
    print(f"UNGEDECKT: {len(gruen)} von {len(ergebnisse)}")
    for f in gruen:
        print(f"  {f}")
    # **Getrennt gezählt, nicht mitgezählt** (AC6 von #1087). Ein Feld, dessen
    # Änderung die Auflösung nie erreicht hat, ist NICHT ungedeckt — über seine
    # Deckung ist gar nichts gemessen worden. Stünde es in derselben Zeile,
    # wiederholte die Wertänderung genau den Fehler, den sie an der Streichung
    # behebt: zwei verschiedene Sachverhalte unter einer Zahl.
    if fraglich:
        print(f"\nNICHT GEMESSEN (Schritt neben dem Raster): {len(fraglich)}")
        for f in fraglich:
            print(f"  {f}")
        print("  Diese Felder sind WEDER gedeckt NOCH ungedeckt — die Änderung"
              f"\n  ist nie angekommen. Erneut fahren mit --delta {RASTER} oder"
              f" -{RASTER}.")
    # **Und die Warnung, die auch für ein sauberes GRÜN gilt.** Der Schritt
    # kann auf dem Raster liegen und die Änderung trotzdem wirkungslos sein —
    # gemessen an `decayWard.durationMs` mit `+10`: Das Fenster wandert von
    # 12000 auf 12010, und der Prüffall, der bei 12000 zuschlägt, bleibt
    # drinnen. Erst `-10` färbt ihn. EIN EINZELNER LAUF KANN „UNGEDECKT" NICHT
    # BELEGEN; er kann es nur behaupten.
    if gruen and args.delta is not None:
        print(f"\n  GEGENRICHTUNG NICHT GEFAHREN. Ein GRUEN bei --delta {args.delta:g}"
              f"\n  trennt nicht zwischen \"das Feld ist ungedeckt\" und \"diese"
              f"\n  RICHTUNG ist wirkungslos\". Erst {-args.delta:g} entscheidet das.")
    if alter_stand is not None:
        vergleich = nachweis["historical_comparison"]
        print(f"\nHistorischer Vergleich: {len(vergleich['new_bau'])} alte Felder jetzt BAU")
        for zeile in vergleich["new_bau"]:
            print(f"  {zeile['field']}: {zeile['old_verdict']} -> BAU (Exit {zeile['build_exit']})")
        if not nachweis["full_pass"]:
            print("KEINE VOLLSTÄNDIGE ABNAHME: Teilmenge, Nur-Bau, Wertänderung oder unvollständige Suite.")
    return 1 if fehler else 0


if __name__ == "__main__":
    sys.exit(main())
