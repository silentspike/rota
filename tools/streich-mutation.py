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
Mit `--delta` kommt GRUEN? dazu: grün, aber keine Entscheidung genau auf dem
Wert belegt — ohne Aussage (AC7 #1087, `--entscheidung`).

EXIT-CODES, damit „nicht gemessen" nie wie ein Befund aussieht:
  0   Durchgang beendet, kein Feld FEHLER oder ABBRUCH
  1   mindestens ein Feld FEHLER (Suite unvollständig, Daten im Lauf verändert),
      oder ein Abbruch mit Ausnahme (Zeitüberschreitung, Rückbau abweichend)
  2   NICHT GESTARTET oder NICHT GELAUFEN: unzulässiger Schritt, unbekanntes
      Feld, verunreinigter Datensatz, unlesbare Zeitregel; oder der Bau-Dienst
      lehnte ab bzw. die Übertragung brach (75, 70, 74, 130, 2, 12, 23, 255).
      Exit 75 erst nach 10 Versuchen im Abstand von 120 s, die übrigen sofort
  3   die Nullprobe oder eine Vorprobe lieferte nicht ihren Ausgang
  143 SIGTERM; die Kindgruppen sind beendet, der Datensatz zurückgestellt

DREI VORPROBEN (#1055, `--vorproben`; der Volllauf fährt sie): die Nullprobe
ohne Mutation (GRÜN), die BAU-Vorprobe (Pflichtfeld `id` gestrichen, muss mit
build.rs-Panic als BAU erkannt werden — genau dort, wo die alte Textregel
nichts fand) und die ROT-Vorprobe (`baseDamage` +1, muss Rust- UND TS-Namen
färben). Ohne sie ist eine leere BAU-Spalte nicht deutbar: „kein Feld bricht
den Bau" und „die Erkennung greift nicht" sähen gleich aus.

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
    opt_uint()          nein                    11          11

Gezählt am 2026-10-28 an cf720d73, nur Code, ohne Definitionen und
Kommentare. Die erste Fassung nannte für `opt_uint()` 12 Aufrufe; an ihrem
eigenen Stand (20336f30) waren es nach derselben Zählung 11.

Die Einheit steht in der Überschrift, nicht in der Zelle — der erste Entwurf
zählte in den ersten zwei Zeilen Aufrufe und in der dritten Feldnamen. Wer ihn
addierte, bekam 74 und meinte, etwas gezählt zu haben. **Das ist genau die
Klasse, die dieser Dateikopf beschreibt, in ihm selbst.**

**Jede dieser Zahlen braucht ihre Definition daneben**, sonst widersprechen
sie einander morgen. Für `uint()` gibt es vier richtige Antworten:

    66   Treffer auf `uint(` an einer Wortgrenze        einschl. der Definition
                                                        und einer Kommentarzeile
    64   echte Aufrufe                                  ohne `fn uint(…)`
    63   Aufrufe mit LITERALEM Feldnamen                <- die brauchbare Zahl
    41   verschiedene Feldnamen                         Aufrufe mehrfach gezählt

Die 66 war am 2026-08-22 noch 65: #1104 schrieb in `emit_putrefaction` einen
Kommentar mit „`uint()` waere falsch". Die Aufrufe blieben, die Regex-Zahl
wanderte — der Grund, warum die brauchbare Zahl die der Aufrufe ist.

Die 63 ist die brauchbare, weil nur ein literaler Feldname sich einem
Tabellenfeld zuordnen lässt. Der eine Ausreißer steht in `emit_fixation`
(sim/build.rs):

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

Das eine bewachte steht in `emit_dot` (sim/build.rs):

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

**Und die EINHEIT entschied, ob `+1` überhaupt etwas bewirken kann** — so
stand es hier bis #1104, und die Tabelle bleibt als Herkunft:

    Aufzählung      `+1` wechselt den FALL             buff.kind [0,1]
    feine Größe     `+1` konnte im Raster verschwinden extendShieldMs (ms)
    grobe Größe     `+1` wirkt immer                   dotExtendTicks (Ticks)

Am 2026-08-22 an `extendShieldMs` (Wert 12000): Die Mutation 12000 → 12001
blieb grün, kein Fall färbte (gemessen). Warum, steht in `combat.ts` (gelesen,
kein Lauf — so auch #1104): Es addiert dort ohne `snapUp` und vergleicht gegen
`seitBeginn`, das nur Vielfache von `TICK_MS` (10) annimmt; 12000 und 12001
laufen damit **beide erst bei 12010** ab. `extendShieldMs` und `extendDecayMs`
waren damals die einzigen `Ms`-Felder ohne Rasterprüfung, in beiden Sprachen.

**SEIT #1104 GIBT ES DIESE MITTLERE ZEILE NICHT MEHR.** a821ceed zog die zwei
Felder aufs Raster (sim/build.rs, src/core/data.ts), d19d7d7e machte daraus
eine Regel für ALLE: `time_fields::validate` (sim/time_fields.rs, in build.rs
für jede Zauberdatei vor dem Übersetzen aufgerufen) und `validateSpellTimes`
(src/core/data.ts) verlangen jedes `…Ms`-Feld auf dem Takt, außer den
ausdrücklich benannten Vergleichsschwellen — heute genau
`cooldownReductionBelowMs`. Von den 16 `Ms`-Feldern der Zauberdaten bricht
damit laut dieser Regel an 15 ein Schritt neben dem Takt den Bau (aus der
Regel gelesen, nicht an 15 Bauten gemessen). Ein solcher Schritt startet hier
gar nicht erst (`schritt_unzulaessig`, Exit 2): Er misst den Bau, nicht die
Deckung. Der kleinste zulässige Schritt an einem Rasterfeld ist ±10.

Die Schwelle bleibt der eine Ort, an dem `+1` zulässig ist und trotzdem
verschwinden kann: `shortenCooldowns` (combat.ts) vergleicht
`readyAt - timeMs >= grenze`. Ein Schritt von `g` auf `g+1` ändert dort nur
etwas, wenn eine Restzeit genau `g` beträgt — die Frage nach der Entscheidung
auf dem Wert.

Die Regel liest dieses Werkzeug nicht selbst, sondern über `policy()` in
tools/spell-time-audit.py (`zeitregel()`); weichen TS- und Rust-Seite
voneinander ab, startet nichts.

**WAS SICH DARAUS NICHT ABLEITEN LÄSST: eine automatische Einteilung.** Ein
Versuch, die 45 Zahlenfelder nach ihren Werten zu sortieren („höchstens zwei
verschiedene, alle unter vier → Aufzählung"), hat sechs Felder falsch
einsortiert — darunter `dotExtendTicks [1]`, `decayCap.bonus [2]` und
`cooldownResetCharges [1,2]`. Ein Feld mit dem Wert 1 ist nicht deshalb eine
Aufzählung. Der Unterschied steht in der Verwendung, nicht in der
Werteverteilung, und wer nach Werten sortiert, misst das Leichte statt des
Nötigen.

`feldart()` ist deshalb KEINE solche Einteilung. Sie nennt, was der Bau an
einem Feld erzwingt (raster, schwelle) und welche Zahlen es trägt (ganzzahl,
bruchzahl, keine-zahl) — und entscheidet damit nur, welcher Schritt zulässig
ist. Ob eine ganze Zahl eine Menge oder eine Aufzählung ist, sagt sie nicht.

**Der Prüfschritt statt der Einteilung**, und er gilt je Feld:

> Liegt eine Entscheidung GENAU auf dem Wert? Bei `decayWard.durationMs`
> (6000) ja — der Prüffall setzt einen Schlag exakt auf 12000, und `<=`
> entscheidet dort. Bei `extendShieldMs` nein. Wenn nein, ist `+1`
> wahrscheinlich wirkungslos, und ein `GRÜN` heißt nichts.

Sechzehn der 45 Zahlenfelder tragen die Endung `Ms`. Gestellt wird die Frage
seit #1087 AC7 bei JEDEM `--delta`-Lauf, vor dem Lauf, im Protokoll:
`--entscheidung FELD=ja:FUNDSTELLE` (oder `nein`) beantwortet sie, und nur mit
„ja" geht ein `GRÜN` als „ungedeckt" in die Tabelle. Ohne Antwort heißt es
`GRUEN?`.

**Nicht die GRÖSSE der Änderung entscheidet, sondern ob sie eine Grenze
überschreitet.** Der Beleg steht in `decayWard.durationMs` (Wert 6000) und
seinem Prüffall in `tests/unit/putrefaction-fenster.test.ts`, „der Schlag bei
12000 ms liegt gerade noch darin". Die Logik dazu, in `combat.ts`:

    gesetzt      decayWardUntilMs = seitBeginn + decayWard.durationMs
    verglichen   seitBeginnFuerDecay <= this.decayWardUntilMs

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

    damals sim/build.rs, im Wortlaut aus der Bauausgabe:
    calcination.json/spells[decay]/decayWard: durationMs 6001 liegt nicht
    auf dem 10-ms-Raster
    (der Dateiname darin war falsch, `decay` steht in putrefaction.json —
    siehe #1104; heute lehnt `time_fields::validate` den Wert vor jeder
    Übersetzung ab, und ein solcher Lauf startet hier gar nicht mehr)

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
zulässig?** Ein Schritt neben dem Raster erzeugte `BAU` — und `BAU` sagt nichts
über die Deckung des Wertes, nur über die Zulässigkeit der Mutation. Seit
#1087 prüft das `schritt_unzulaessig` vor dem Lauf.

**`+10` gefahren, und das Ergebnis ist der Fall, vor dem dieser Modus warnt:**

    decay: 6000->6010    GRÜN    beide Suiten grün    3,2 min
                                 Werkzeug schreibt „UNGEDECKT: 1 von 1"

Das Feld ist **nicht** ungedeckt. Der Prüffall steht direkt darauf — er sieht
nur diese Richtung nicht: 12000 <= 12010 lässt den Schlag weiter hinein.
`--delta -10` färbt dieselbe Zeile.

**Daraus folgten VIER Ausgänge, von denen zwei gleich aussahen:**

    BAU     die Änderung ist unzulässig     gerastertes Feld, ±1
    GRÜN    die Änderung ist wirkungslos    decayWard.durationMs, +10
    GRÜN    das Feld ist ungedeckt          der gesuchte Fall
    ROT     das Feld ist gedeckt

Heute startet die erste Zeile nicht mehr (Exit 2), und die beiden `GRÜN` sind
im Bericht getrennt: `GRUEN` nur mit belegter Entscheidung auf dem Wert,
sonst `GRUEN?` unter OHNE AUSSAGE. Endgültig trennt sie trotzdem erst die
**Gegenrichtung**: Wird dasselbe Feld in die andere Richtung rot, war das
`GRÜN` Wirkungslosigkeit. Bleibt es auch dort grün, ist der Wert wirklich
ungedeckt. Ein einzelner `--delta`-Lauf kann „ungedeckt" nicht belegen — er
kann es nur behaupten.

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
**Bedingung dafür, dass die Messung überhaupt misst** — keine Politur. Er
greift bei JEDEM Lauf, denn `--delta` liest der Aufruf als Gleitkommazahl.
Ein Schritt mit Nachkommastellen an einem Ganzzahlfeld startet seit #1087
gar nicht erst (Exit 2).

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
import importlib.util
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


def zeitregel():
    """Takt und Schwellen der Zeitfelder — aus `policy()`, nicht nachgebaut (#1104).

    Seit #1104 gibt es dafür EINE Produktionsregel in zwei Sprachen:
    `sim/time_fields.rs` und `src/core/data.ts` verlangen jedes `…Ms`-Feld auf
    dem Takt, außer den ausdrücklich benannten Vergleichsschwellen.
    `policy()` in tools/spell-time-audit.py liest beide Seiten samt `TICK_MS`
    (src/core/loop.ts) und `RASTER_MS` (sim/build.rs) und bricht ab, wenn sie
    auseinandergehen. Bis zu diesem Umbau las dieses Werkzeug nur die zwei
    Konstanten selbst (`raster_ms()`) — eine zweite Lesart derselben Regel,
    die die Schwellen nicht kannte.

    Ein Abweichen ist ein Befund über den Bestand, keine Vorgabe: Der Aufruf
    bricht ab, statt eine Seite zu bevorzugen.
    """
    spec = importlib.util.spec_from_file_location(
        "spell_time_audit", pathlib.Path(__file__).with_name("spell-time-audit.py"))
    audit = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(audit)
    try:
        takt, schwellen = audit.policy(WURZEL)
    except (OSError, ValueError) as grund:
        raise NichtGestartet(f"Zeitregel nicht verwendbar (#1104): {grund}. "
                             "Das ist ein Befund über den Bestand, keine Vorgabe.")
    return takt, frozenset(schwellen)


FELDARTEN = ("raster", "schwelle", "ganzzahl", "bruchzahl", "keine-zahl")


def feldart(feld, werte, regel):
    """Die Eigenschaft eines Feldes — aus der Produktionsregel und den Daten (#1087 AC6).

    Keine Deutung, was ein Feld BEDEUTET: Eine Einteilung nach der
    Werteverteilung hat sechs von 45 Feldern falsch sortiert (Dateikopf). Hier
    steht nur, was der Bau an einem Feld erzwingt und welche Zahlen es trägt:

        raster      `…Ms`, auf dem Takt erzwungen     time_fields.rs, data.ts
        schwelle    `…Ms`, ausdrücklich ungerastert   die Ausnahmen in policy()
        ganzzahl    jeder Wert im Bestand ganzzahlig
        bruchzahl   mindestens ein Wert mit Nachkommastellen
        keine-zahl  Text, Wahrheitswert, Liste

    Ob eine ganze Zahl eine MENGE ist oder eine AUFZÄHLUNG (`buff.kind`), steht
    hier nicht und lässt sich so nicht ablesen — dafür ist die Frage nach der
    Entscheidung auf dem Wert da (`--entscheidung`).
    """
    _takt, schwellen = regel
    name = feld.split(".")[-1]
    if name.endswith("Ms"):
        return "schwelle" if name in schwellen else "raster"
    if werte and all(type(w) is int for w in werte):
        return "ganzzahl"
    if werte and all(type(w) in (int, float) for w in werte):
        return "bruchzahl"
    return "keine-zahl"


def schritt_unzulaessig(art, delta, takt):
    """Warum `delta` an einem Feld dieser Art die Deckung nicht messen KANN — oder None.

    Jeder dieser Läufe endete bekannt, bevor eine Prüfung den Wert sähe: Der
    Bau bricht, und `BAU` hieße dann weder „Pflichtfeld" noch „ungedeckt",
    sondern „unzulässige Mutation". Deshalb startet er gar nicht erst.
    """
    if art == "keine-zahl":
        return "kein Zahlenfeld — eine Wertänderung ist hier nicht definiert"
    if delta == 0:
        return "Schritt 0 ändert nichts"
    if art != "bruchzahl" and not float(delta).is_integer():
        return (f"Schritt {delta:g} ist nicht ganzzahlig — build.rs liest Zahlenfelder mit "
                "uint()/opt_uint() und lehnt Fließkomma ab; gemessen würde der Typ, nicht die Deckung")
    if art == "raster" and int(delta) % takt:
        return (f"Schritt {delta:g} liegt neben dem {takt}-ms-Takt — seit #1104 lehnen "
                "sim/time_fields.rs und src/core/data.ts jeden solchen Wert ab; gemessen "
                f"würde der Bau, nicht die Deckung. Kleinster zulässiger Schritt: ±{takt}")
    return None


def aufloesung(art, takt):
    """Die Größe, gegen die ein Schritt an dieser Feldart wirkt, soweit bekannt (AC6)."""
    if art == "raster":
        return f"Takt {takt} ms"
    if art == "schwelle":
        # src/core/combat.ts, shortenCooldowns: `readyAt - timeMs >= grenze` —
        # ein Schritt unter dem Takt wirkt nur, wenn eine Restzeit genau darauf fällt.
        return f"Vergleichsschwelle readyAt - timeMs >= Wert, Takt {takt} ms"
    return None


FRAGE = "Liegt eine Entscheidung genau auf dem Wert?"


def entscheidungen_lesen(angaben, felder):
    """`--entscheidung FELD=ja:FUNDSTELLE` oder `=nein:…` → {feld: (antwort, fundstelle)}.

    Die zweite Rückgabe sind die Einwände; ist sie nicht leer, startet nichts.
    """
    ergebnis, einwaende = {}, []
    for angabe in angaben or []:
        feld, gleich, rest = angabe.partition("=")
        antwort, doppelpunkt, fundstelle = rest.partition(":")
        if not gleich or not doppelpunkt or antwort not in ("ja", "nein") or not fundstelle.strip():
            einwaende.append(f"'{angabe}': erwartet FELD=ja:FUNDSTELLE oder FELD=nein:FUNDSTELLE")
        elif feld not in felder:
            einwaende.append(f"'{angabe}': {feld} wird in diesem Lauf nicht gefahren")
        elif feld in ergebnis:
            einwaende.append(f"'{angabe}': {feld} ist doppelt beantwortet")
        else:
            ergebnis[feld] = (antwort, fundstelle.strip())
    return ergebnis, einwaende


def fingerabdruecke(pfade):
    """SHA-256 je Zauberdatei, relativ zur Wurzel benannt."""
    return {str(p.relative_to(WURZEL)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in pfade.values()}


def gesamtabdruck(abdruecke):
    """Ein Abdruck über alle Dateien — für die Protokollzeile; der Nachweis hält jede einzeln."""
    zeilen = "".join(f"{name} {wert}\n" for name, wert in sorted(abdruecke.items()))
    return hashlib.sha256(zeilen.encode()).hexdigest()[:12]


@contextmanager
def gesicherter_datensatz(eintrag):
    """Sicherung, Fingerabdrücke und byteweiser Rückbau der sechs Zauberdateien.

    Sicherungskopien ALLER Schulen, nicht nur der betroffenen: Ein Feld kann in
    mehreren stehen (control.cost in fixation, heal.amount in coagulation), und
    ein halb zurückgesetzter Datensatz verfälscht jeden folgenden Lauf. Feld
    und Vorprobe nehmen denselben Weg — zwei Kopien dieser Zeilen gingen
    auseinander, und am Rückbau wird das teuer.

    FINGERABDRÜCKE an vier Stellen (AC4 #1087), je Datei in `eintrag["sha256"]`:
        before          der Stand, gegen den die Nullprobe gemessen hat
        mutated         was die Suiten sehen sollen (setzt der Aufrufer)
        before_restore  was sie gesehen HABEN (setzt der Aufrufer nach dem
                        Lauf, sonst hier) — weicht es von `mutated` ab, hat
                        während des Laufs jemand geschrieben
        after_restore   muss `before` gleichen; `restored` sagt, ob es das tut
    """
    pfade = pfade_aller_schulen()
    sicherung = {s: p.read_bytes() for s, p in pfade.items()}
    abdruecke = eintrag["sha256"] = {"before": fingerabdruecke(pfade)}
    try:
        yield pfade, abdruecke
    finally:
        if "mutated" in abdruecke and "before_restore" not in abdruecke:
            abdruecke["before_restore"] = fingerabdruecke(pfade)
        for schule, p in pfade.items():
            p.write_bytes(sicherung[schule])
        abdruecke["after_restore"] = fingerabdruecke(pfade)
        eintrag["restored"] = (all(p.read_bytes() == sicherung[s] for s, p in pfade.items())
                               and abdruecke["after_restore"] == abdruecke["before"])
        if "mutated" in abdruecke:
            print(f"      sha256 vorher {gesamtabdruck(abdruecke['before'])}"
                  f"  mutiert {gesamtabdruck(abdruecke['mutated'])}"
                  f"  vor Rückbau {gesamtabdruck(abdruecke['before_restore'])}"
                  f"  nachher {gesamtabdruck(abdruecke['after_restore'])}"
                  f" — {'Rückbau byteweise identisch' if eintrag['restored'] else 'RÜCKBAU ABWEICHEND'}",
                  flush=True)


def _liste(daten):
    return daten if isinstance(daten, list) else daten["spells"]


def vorprobe_bau(pfade):
    """Streicht das Pflichtfeld `id` im ersten Zauber von common.json.

    `id` ist Stammdatum, kein Wirkfeld, und build.rs liest es mit `text()`,
    das bei fehlendem Feld panickt — der Bau MUSS brechen, mit Exit 101 und
    ohne Compilermeldung. Genau diesen Fall hat Worker-1 bei #1045 hergestellt
    und die alte Textregel übersehen (#1055).
    """
    p = pfade["common"]
    daten = json.loads(p.read_text())
    zauber = _liste(daten)[0]
    sid = zauber.pop("id")
    p.write_text(json.dumps(daten, indent=2, ensure_ascii=False) + "\n")
    return f"common.json: Pflichtfeld id ('{sid}') im ersten Zauber gestrichen"


def vorprobe_rot(pfade):
    """+1 auf `baseDamage` des ersten Zaubers, der einen positiven trägt (Schulreihenfolge).

    Ein Grundwert, den jede Schadensrechnung liest, auf beiden Seiten: Färbt
    hier nicht mindestens ein Rust- UND ein TS-Fall, sammelt der Durchgang die
    Namen einer Seite nicht ein — und ein ROT stünde auf Namen der anderen.
    """
    for schule in SCHULEN:
        p = pfade[schule]
        daten = json.loads(p.read_text())
        for zauber in _liste(daten):
            alt = zauber.get("baseDamage")
            if type(alt) is int and alt > 0:
                zauber["baseDamage"] = alt + 1
                p.write_text(json.dumps(daten, indent=2, ensure_ascii=False) + "\n")
                return f"{schule}.json: {zauber.get('id', '?')} baseDamage {alt}->{alt + 1}"
    raise NichtGestartet("ROT-Vorprobe nicht möglich: kein Zauber trägt einen positiven baseDamage")


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


def blattwerte():
    """Alle Wirkfelder als Pfad -> Liste ihrer Werte, über dieselbe Auswahl wie blattfelder()."""
    werte = collections.defaultdict(list)

    def geh(praefix, obj):
        for k, v in obj.items():
            if k in STAMM or (not praefix and k in GRUNDWERTE):
                continue
            if isinstance(v, dict):
                geh(f"{praefix}{k}.", v)
            else:
                werte[f"{praefix}{k}"].append(v)

    for schule in SCHULEN:
        for s in spells(schule):
            geh("", s)
    return werte


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

    Die Ausgänge in DIENST_AUSGAENGE kommen hier nie an — `stufe()` hält sie
    vorher an, denn sie heißen „nicht gebaut", nicht „Bau gebrochen".
    """
    return code != 0 or "error: could not compile" in ausgabe or "error[E" in ausgabe


# Diese Ausgänge stammen vom cargo-remote-v2-CLIENT, nicht von cargo: Gebaut
# oder gemessen wurde nichts, oder das Ergebnis kam nicht an. cargo selbst endet
# bei Bau- und Testfehlern mit 101.
#
# Gemessen am 2026-10-23: Ein echter Abweis — Exit 75, "This workspace already
# has an active job" — galt `bau_gebrochen()` als gebrochener Bau. Im Feldlauf
# hätte das Feld damit BAU geheißen, ohne dass ein einziger Compiler lief. Und
# seit v2 parallele Worktrees zulässt, ist Exit 75 ein Alltagsfall: Der Server
# reiht nichts ein, er lehnt ab, wenn sein Budget voll ist.
#
# 12, 23 und 255 kommen gar nicht vom Client selbst, sondern von rsync, über
# das er die Quellen hochlädt: `subprocess.run(rsync, check=True)` und am Ende
# `sys.exit(exc.returncode)` (client.py der installierten v2-Fassung). Der
# Code wird also unverändert durchgereicht. Ohne diese Einträge hieß ein
# abgebrochener Upload im Feld-Bau BAU — ein Feld als Pflichtfeld verbucht,
# für das kein einziger Compiler lief (#1055).
DIENST_AUSGAENGE = {
    75: "kein Platz auf dem Bau-Dienst oder Arbeitsbereich belegt — nichts eingereiht",
    70: "Verbindung oder Ausführer des Bau-Dienstes verloren",
    74: "Artefakte nicht übertragbar",
    130: "abgebrochen",
    2: "Aufruf oder Projektkonfiguration vom Client abgelehnt",
    12: "rsync: Fehler im Protokoll-Datenstrom — Übertragung abgebrochen",
    23: "rsync: Teilübertragung — nicht alle Dateien übertragen",
    255: "ssh unter rsync gescheitert — keine Verbindung zum Bau-Dienst",
}

# **Nur Exit 75 wird wiederholt** (#1055). Ohne Wiederholung beendete EIN
# Abweis den ganzen Durchgang von drei bis vier Stunden. Am 29.10. hielt ein
# fremder Auftrag 4 der 13 GiB des Dienstes, und sechs Pakete bauten parallel —
# bei 60 Feldern mit je zwei Dienst-Schritten ist ein Abweis dann fast sicher.
#
# 75 ist die Absage des Dienstes: Budget voll, der Arbeitsbereich hat schon
# einen aktiven Auftrag, das Werkzeugabbild wird eingerichtet, weniger als 20 %
# Platz, oder die Quellen änderten sich beim Hochladen. Gelesen in der
# installierten v2-Fassung (20261004-r30-approved): Auf jedem dieser Wege bleibt
# kein Auftrag aktiv. `reserve()` lehnt ab, bevor etwas eingetragen ist
# (common.py); nach der Reservierung nimmt der Client sie selbst zurück,
# solange nichts gestartet ist (`cancel` in client.py). Ein neuer Versuch kann
# also keinen zweiten Auftrag neben einen ersten stellen.
#
# Die übrigen DIENST_AUSGAENGE bleiben ohne Wiederholung: 70 heißt Verbindung
# oder Ausführer verloren, der Auftrag kann zurückgehalten sein (`--resume`),
# und ein neuer Versuch stellte womöglich einen zweiten daneben. Die anderen
# sind Störungen, deren Ursache erst jemand ansehen muss.
#
# 120 s und 10 Versuche sind die Regel aller Pakete für Exit 75; als
# Konstanten, damit die Prüffälle die Wartezeit verkürzen können.
WIEDERHOLBAR = 75
DIENST_WARTEN_S = 120
DIENST_VERSUCHE = 10


def dienst_meldung(ausgabe):
    """Die Absage in einer Zeile: der Client schreibt `cargo-remote: <Grund>`."""
    zeilen = [z.strip() for z in ausgabe.splitlines() if z.strip()]
    absagen = [z for z in zeilen if z.startswith("cargo-remote:")]
    return (absagen or zeilen or ["(keine Ausgabe)"])[-1][:200]


class NichtGelaufen(RuntimeError):
    """Der Bau-Dienst hat den Schritt nicht ausgeführt; es gibt nichts zu verbuchen."""


class NichtGestartet(RuntimeError):
    """Eine Vorbedingung fehlt; gemessen wurde nichts. Exit 2, nicht 1.

    `raise SystemExit(text)` endete mit Exit 1 — dem Code für „ein Befund".
    """


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
            # cargo-remote führt stdout/stderr zusammen. Eine Running-Zeile
            # kann dadurch direkt hinter einer fertigen Suite-Summe stehen.
            # Den Marker trotzdem zählen; fehlende Targets bleiben ein Fehler.
            targets = re.findall(r"\bRunning [^\n]+? \([^\n)]+\)", text)
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
            unbehandelt = re.search(
                r"^\s*(?:Errors\s+[1-9]\d* errors?\b|"
                r"Vitest caught [1-9]\d* unhandled errors?\b|"
                r"Vitest found [1-9]\d* errors? not related to your test files)",
                text, re.M)
            vollstaendig = bool(werte) and not unbehandelt and code in (0, 1)
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
    # Ohne bestandene Vorproben ist auch ein vollständiger Durchgang keine
    # Abnahme: Eine leere BAU-Spalte wäre dann nicht deutbar (#1055).
    return {"historical_fields": len(alt), "new_bau": neu_bau,
            "unmeasured_old_fields": ungemessen,
            "new_fields": sorted(set(nachweis["all_fields"]) - set(alt)),
            "full_pass": (nachweis.get("full_pass", False) and not ungemessen
                          and nachweis.get("probes_passed", False))}


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
        "**GRUEN?** heißt (nur mit `--delta`): Beide Suiten blieben grün, aber keine",
        "Entscheidung genau auf dem Wert ist belegt (`--entscheidung FELD=ja:…`",
        "fehlt oder lautet `nein`). Die Änderung kann wirkungslos gewesen sein —",
        "weder gedeckt noch ungedeckt. Ein Schritt neben dem Takt an einem",
        "Rasterfeld startet seit #1104 gar nicht erst (Exit 2).",
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


def _warten(sekunden):
    """Wartet in kurzen Schritten — SIGTERM und ein Abbruch sollen nicht 120 s liegen.

    **Nicht `time.sleep(sekunden)` am Stück.** Seit PEP 475 setzt Python einen
    vom Signal unterbrochenen Schlaf fort, wenn der Handler nicht wirft, und
    `_abbruch` wirft absichtlich nicht. Der Durchgang endete dann erst nach der
    ganzen Wartezeit. systemd etwa schickt nach 90 s SIGKILL hinterher (Vorgabe
    von `TimeoutStopSec`), und bei SIGKILL wird nichts zurückgestellt.
    """
    ende = time.monotonic() + sekunden
    while True:
        _unterbrechung_pruefen()
        rest = ende - time.monotonic()
        if rest <= 0:
            return
        time.sleep(min(rest, 0.2))


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


# Wie lange `_gruppe_beenden` nach SIGKILL auf die übrigen Mitglieder wartet.
# Ein Python-Enkel brauchte bis zum Zombie 1 bis 2 ms (gemessen 28.10.); die
# Frist gilt dem Prozess, der in einem nicht unterbrechbaren Aufruf hängt.
GRUPPEN_FRIST_S = 5


def _lebende_mitglieder(gruppe):
    """PIDs der Prozessgruppe, von denen noch ein Thread läuft; None ohne /proc.

    **`p.wait()` sammelt nur das eigene Kind ein, und das ist die Hülle.** Die
    Arbeit tun ihre Kinder: dash 0.5.12, das `/bin/sh` der CI, forkt für
    `sh -c 'cmd'` und ersetzt sich nicht per exec (gemessen: das Kommando bekam
    eine eigene PID); npx und cargo remote starten ohnehin weitere Prozesse.
    Sie bekommen SIGKILL, sterben aber danach, nicht davor: Ein Enkel stand in
    50 von 50 Läufen noch auf `R`, als `_alle_beenden()` zurückkehrte — und
    gleich danach stellte das Werkzeug die Daten zurück. Daher wackelte
    `test_sigterm_stops_both_real_children_and_joins_before_restoring`
    (`'R' != 'Z'`) nur unter Last: Dort reichte das Abwickeln nicht mehr, um
    den Enkel vorher sterben zu lassen.

    Ein Zombie (`Z`) zählt als beendet: Er führt nichts mehr aus, und
    einsammeln kann ihn nur sein neuer Elternprozess, nicht dieses Werkzeug.
    Geprüft wird jeder Thread, nicht nur der erste — der erste Thread eines
    Prozesses kann schon Zombie sein, während andere noch sterben.
    """
    try:
        eintraege = os.listdir("/proc")
    except OSError:
        return None
    lebend = []
    for name in eintraege:
        if not name.isdigit():
            continue
        try:
            with open(f"/proc/{name}/stat") as f:
                # Der Prozessname kann Leerzeichen und Klammern tragen; erst
                # hinter der LETZTEN ") " stehen Zustand, Eltern, Gruppe.
                felder = f.read().rpartition(") ")[2].split()
            if len(felder) < 3 or int(felder[2]) != gruppe:
                continue
            threads = os.listdir(f"/proc/{name}/task")
        except OSError:  # beendet und eingesammelt, während wir lasen
            continue
        for tid in threads:
            try:
                with open(f"/proc/{name}/task/{tid}/stat") as f:
                    zustand = f.read().rpartition(") ")[2][:1]
            except OSError:  # dieser Thread ist schon fort
                continue
            if zustand not in ("Z", "X"):
                lebend.append(int(name))
                break
    return lebend


def _gruppe_beenden(p):
    """Beendet die Prozessgruppe eines Kindes, erst freundlich, dann hart.

    Kehrt erst zurück, wenn kein Mitglied der Gruppe mehr läuft — nicht schon,
    wenn das eigene Kind eingesammelt ist. Der Rückbau folgt unmittelbar.
    """
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
    frist = time.monotonic() + GRUPPEN_FRIST_S
    while True:
        lebend = _lebende_mitglieder(p.pid)
        if not lebend or time.monotonic() >= frist:
            break
        time.sleep(0.005)
    if lebend:
        # Nicht abbrechen: Ohne Rückbau bliebe die Mutation im Baum liegen.
        print(f"WARNUNG: Prozessgruppe {p.pid} läuft {GRUPPEN_FRIST_S} s nach SIGKILL noch "
              f"(PID {', '.join(map(str, lebend))}). Der Rückbau folgt trotzdem.",
              file=sys.stderr)
    elif lebend is None:
        print(f"HINWEIS: /proc nicht lesbar — ob Prozessgruppe {p.pid} vor dem Rückbau "
              "geendet hat, ist nicht geprüft.", file=sys.stderr)


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
    ap.add_argument("--feld", action="append",
                    help="nur dieses Feld (Trägerprobe); mehrfach für mehrere")
    ap.add_argument("--art", choices=FELDARTEN,
                    help="nur Felder dieser Art (siehe feldart(); Zeitregel aus policy())")
    ap.add_argument("--ab", help="erst ab diesem Feld beginnen")
    ap.add_argument("--entscheidung", action="append", metavar="FELD=ja|nein:FUNDSTELLE",
                    help="nur mit --delta: die Antwort auf 'Liegt eine Entscheidung genau "
                         "auf dem Wert?' samt Fundstelle (AC7 #1087). Ohne 'ja' heißt "
                         "ein GRÜN 'GRUEN?' — es sagt dann nichts über die Deckung.")
    ap.add_argument("--vorproben", action="store_true",
                    help="vor den Feldern die BAU- und die ROT-Vorprobe fahren "
                         "(positive Kontrollen der Erkennung, #1055); der Volllauf braucht sie")
    ap.add_argument("--aus", default="docs/streich-mutation-ergebnis.md")
    ap.add_argument("--roh", default="/work/tmp/e1-mutation-namen.json",
                    help="alle roten Testnamen je Feld, Grundlage der Auswertung")
    ap.add_argument("--nachweis", help="vollständiger JSON-Nachweis; sonst --aus mit Endung .json")
    ap.add_argument("--vergleich", help="historischer Bericht für den namentlichen BAU-Abgleich")
    ap.add_argument("--bau-log", default=BAU_LOG, help="Bauprotokoll dieses isolierten Laufs")
    ap.add_argument("--timeout", type=int, default=1800,
                    help="Grenze in Sekunden je Prozess einer Stufe; nach Exit 75 bekommt "
                         "jeder neue Versuch sie neu, die Wartezeit zählt nicht mit")
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

    # NUR bei `--delta` oder `--art` gelesen: Ohne Wertänderung gibt es keine
    # Schrittweite, gegen die etwas zu prüfen wäre — und ein Abbruch wegen
    # einer umbenannten Konstante träfe dann einen Lauf, der sie nicht braucht.
    regel = zeitregel() if args.delta is not None or args.art else None
    TAKT = regel[0] if regel else None

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
    # Zwei REMOTE-Schritte nacheinander: zuerst nur bauen, dann dort testen.
    # Nacheinander schon deshalb, weil cargo remote v2 je Arbeitsverzeichnis nur
    # EINEN aktiven Job zulässt — ein zweiter bekäme Exit 75 (gemessen am
    # 2026-10-23). Keine lokalen Rust-Binaries starten, keine Artefakte und
    # insbesondere keine Cargo.lock vom Buildserver kopieren.
    # Cargo gibt standardmäßig alle Fallnamen aus; --no-fail-fast verhindert,
    # dass ein rotes Target die Prüfung der übrigen Targets abschneidet.
    rust_bau = "cargo remote --no-copy-lock -- test --no-run --all-targets"
    bau_log = WURZEL / args.bau_log
    rust_lauf = "cargo remote --no-copy-lock -- test --all-targets --no-fail-fast"
    # Dateisystem-/AST-Prüfungen überschritten unter Hostlast die Vitest-
    # Vorgabe von 5 s. Nur die Laufzeitgrenze erhöhen, keine Fälle auslassen.
    ts = "npx --no-install vitest run --testTimeout 30000"

    wo = blattfelder()
    # SELTENE FELDER ZUERST. Ein Feld, das 57 Zauber tragen, ist mit Sicherheit
    # gedeckt; die Frage stellt sich bei den Einzelstücken (`clearsDoom`,
    # `refundsInterruptedGcd`, `doublesNextCast`). Bricht der Lauf nach Stunden
    # ab, liegen dann die Antworten vor, auf die es ankommt.
    felder = sorted(wo, key=lambda f: (len(wo[f]), f))
    werte = blattwerte()
    arten = {f: feldart(f, werte[f], regel) for f in wo} if regel else {}
    if args.feld:
        fremd = [f for f in args.feld if f not in wo]
        if fremd:
            print(f"FEHLER: {', '.join(fremd)} ist kein Wirkfeld. Bekannt sind {len(felder)}.")
            return 2
        felder = list(dict.fromkeys(args.feld))
    elif args.ab:
        felder = felder[felder.index(args.ab):]
    if args.art:
        felder = [f for f in felder if arten[f] == args.art]
    if not felder:
        print("FEHLER: keine Wirkfelder gefunden — ein leerer Durchgang ist keine Messung.")
        return 2

    # ALLE EINWÄNDE VOR DEM ERSTEN LAUF, Exit 2 wie jedes „nicht gemessen".
    # Ein Durchgang, der gemessene und unmessbare Felder mischt, hätte Löcher,
    # die in der Tabelle wie Ergebnisse aussehen; deshalb verweigert ein
    # einziges unzulässiges Feld den ganzen Durchgang. `--art` wählt aus.
    entscheidungen, einwaende = entscheidungen_lesen(args.entscheidung, felder)
    if args.entscheidung and args.delta is None:
        einwaende.append("--entscheidung gilt nur mit --delta: die Frage betrifft eine Wertänderung")
    if args.delta is not None:
        einwaende += [f"{f} ({arten[f]}): {grund}" for f in felder
                      if (grund := schritt_unzulaessig(arten[f], args.delta, TAKT))]
    if einwaende:
        print("NICHT GESTARTET — kein Lauf, keine Mutation:")
        for einwand in einwaende:
            print(f"  {einwand}")
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
                "service_refusals": [],
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
        """Eine Stufe; ein Dienst-Schritt mit Exit 75 wird bis zu DIENST_VERSUCHE-mal gefahren.

        Nullprobe, Vorproben und Felder kommen alle hier durch, also gilt die
        Wiederholung für alle drei gleich.

        **Die Wartezeit zählt nicht in die Stufengrenze.** `--timeout` geht an
        jeden Versuch einzeln, gewartet wird außerhalb von `lauf()`. Die Grenze
        ist für einen hängenden Prozess da (`channelTickMs`: Endlosschleife);
        wer wartet, hängt nicht, und das Warten ist durch die Konstanten
        begrenzt — höchstens neunmal 120 s. Eine gemeinsame Grenze müsste um
        diese 18 min wachsen und griffe bei einem hängenden Prozess entsprechend
        später.
        """
        log = protokolle / f"{probe}.{name}.log"
        dienst = name in ("build", "rust")
        for versuch in range(1, DIENST_VERSUCHE + 1):
            if versuch > 1:
                _warten(DIENST_WARTEN_S)
            begonnen, uhr = time.time(), time.monotonic()
            try:
                code, ausgabe = lauf(befehl, args.timeout)
            except subprocess.TimeoutExpired:
                log.write_text("TIMEOUT: Prozessgruppe beendet; diese Stufe ist nicht abgeschlossen.\n")
                raise
            if name == "build" and bau_log.exists():
                ausgabe += bau_log.read_text()
            log.write_text(ausgabe)
            if not (dienst and code == WIEDERHOLBAR):
                break
            # Jeder Abweis mit Zeit und Meldung: in die Ausgabe (beim Volllauf
            # `lauf.log`), in den Nachweis und mit der vollen Antwort daneben.
            meldung = dienst_meldung(ausgabe)
            uhrzeit = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(begonnen))
            nachweis["service_refusals"].append({
                "stage": f"{probe}/{name}", "attempt": versuch, "of": DIENST_VERSUCHE,
                "started_unix": begonnen, "exit": code, "message": meldung})
            with (protokolle / f"{probe}.{name}.abgewiesen.log").open("a") as abweise:
                abweise.write(f"=== Versuch {versuch}/{DIENST_VERSUCHE}, {uhrzeit}, Exit {code} ===\n"
                              f"{ausgabe}\n")
            speichern()
            print(f"      Bau-Dienst wies ab: {probe}/{name}, Versuch {versuch}/{DIENST_VERSUCHE}, "
                  f"{uhrzeit}, Exit {code} — {meldung}. "
                  + ("Kein weiterer Versuch." if versuch == DIENST_VERSUCHE else
                     f"Neuer Versuch in {DIENST_WARTEN_S:g} s."),
                  flush=True)
        if dienst and code in DIENST_AUSGAENGE:
            # Kein Urteil über das Feld: Der Lauf hat nicht stattgefunden.
            versuche = (f" — nach {versuch} Versuchen im Abstand von {DIENST_WARTEN_S:g} s"
                        if code == WIEDERHOLBAR else "")
            raise NichtGelaufen(f"{probe}/{name}: Exit {code} — {DIENST_AUSGAENGE[code]}{versuche}. "
                                f"Protokoll: {log}")
        eintrag = ({"exit": code, "status": "FAIL" if bau_gebrochen(code, ausgabe) else "PASS"}
                   if name == "build" else suite_nachweis(code, ausgabe, name))
        eintrag.update(log=str(log), started_unix=begonnen, seconds=time.monotonic() - uhr)
        if versuch > 1:
            eintrag["attempts"] = versuch
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

    # DREI VORPROBEN (#1055): die Nullprobe oben, dazu BAU und ROT. Eine leere
    # BAU-Spalte hieß bisher entweder „kein Feld bricht den Bau" oder „das
    # Muster hat nie gegriffen", und aus der Datei war das nicht zu trennen.
    # Erst eine positive Kontrolle IM SELBEN LAUF trennt es: Bricht ein
    # bekannter Pflichtfeld-Abbruch hier als BAU, kann ein Feld ohne BAU nicht
    # an der Erkennung gelegen haben. Dasselbe für ROT und die Rust-Namen.
    # Eine Vorprobe mit falschem Ausgang beendet den Durchgang vor dem ersten
    # Feld, Exit 3 wie die Nullprobe.
    if args.vorproben:
        nachweis["probes"], nachweis["probes_passed"] = {}, False
        for name, aendern, erwartet in (("bau", vorprobe_bau, "BAU"), ("rot", vorprobe_rot, "ROT")):
            eintrag = nachweis["probes"][name] = {"status": "UNGEPRUEFT", "expected": erwartet}
            if args.nur_bau and name == "rot":
                eintrag.update(status="SKIPPED", reason="build-only")
                continue
            aus_r = aus_t = ""
            try:
                with gesicherter_datensatz(eintrag) as (pfade, abdruecke):
                    eintrag["mutation"] = aendern(pfade)
                    abdruecke["mutated"] = fingerabdruecke(pfade)
                    print(f"Vorprobe {erwartet}: {eintrag['mutation']}", flush=True)
                    if args.nur_bau:
                        rc_bau, aus_bau, eintrag["build"] = bauen(f"vorprobe-{name}")
                    else:
                        rc_bau, aus_bau, _, aus_r, _, aus_t = messen(f"vorprobe-{name}", eintrag)
                    abdruecke["before_restore"] = fingerabdruecke(pfade)
            except BaseException as error:
                eintrag.update(status="ABBRUCH", detail=f"{type(error).__name__}: {error}")
                raise
            finally:
                speichern()
            if not eintrag["restored"]:
                raise RuntimeError(f"Rückbau nach der {erwartet}-Vorprobe nicht byteweise identisch")
            gebrochen = bau_gebrochen(rc_bau, aus_bau)
            if name == "bau":
                # Die Stelle, die #1055 meint: Exit 101 aus einem build.rs-Panic,
                # ohne eine der beiden Compilermeldungen.
                eintrag["old_rule_detects"] = "error: could not compile" in aus_bau or "error[E" in aus_bau
                stand = ("baut" if not gebrochen else
                         "BAU" if "panicked at" in aus_bau else "BAU ohne build.rs-Panic")
            else:
                f_r, f_t = rote_namen(aus_r)[0], rote_namen(aus_t)[0]
                eintrag["failure_names"] = {"rust": f_r, "ts": f_t}
                stand = ("BAU" if gebrochen else
                         "FEHLER" if any(eintrag[s]["status"] == "ERROR" for s in ("rust", "ts")) else
                         "ROT" if f_r and f_t else
                         "ROT nur TS" if f_t else "ROT nur Rust" if f_r else "GRUEN")
            if abdruecke["before_restore"] != abdruecke["mutated"]:
                stand = "FEHLER: Datensatz während der Vorprobe verändert"
            eintrag["status"] = stand
            speichern()
            if stand != erwartet:
                print(f"ABBRUCH: Die {erwartet}-Vorprobe ergab „{stand}“. Die Erkennung ist in "
                      "diesem Lauf nicht belegt — jedes Feldergebnis wäre unbelegt, eine "
                      "leere Spalte nicht deutbar.")
                return 3
            print(f"Vorprobe {erwartet}: wie erwartet"
                  + (f" (Exit {rc_bau}, build.rs-Panic; die alte Textregel "
                     f"{'fand ihn' if eintrag['old_rule_detects'] else 'hätte nichts gefunden'})"
                     if name == "bau" else
                     f" ({len(eintrag['failure_names']['rust'])} Rust-, "
                     f"{len(eintrag['failure_names']['ts'])} TS-Namen)"), flush=True)
        nachweis["probes_passed"] = True
        speichern()
        print("Vorproben bestanden.\n", flush=True)

    # Die Kopfzeile nennt, WAS gefahren wird — im Nur-Bau-Modus ist das etwas
    # anderes, und ein Protokoll, das "je ein Lauf beider Suiten" behauptet,
    # während keine Suite läuft, ist später nicht mehr auseinanderzuhalten.
    print(f"{len(felder)} Felder, "
          f"{'nur Bau, keine Suite' if args.nur_bau else 'je ein Lauf beider Suiten'}\n",
          flush=True)
    ergebnisse = []
    roh = {}
    rueckbau_kaputt = None
    for i, feld in enumerate(felder, 1):
        _unterbrechung_pruefen()
        teile = feld.split(".")
        if rueckbau_kaputt:
            raise RuntimeError(f"Rückbau nach {rueckbau_kaputt} nicht byteweise identisch — "
                               "jedes weitere Feld würde gegen fremde Daten gemessen")
        eintrag = {"status": "UNGEPRUEFT", "mutated_spells": 0}
        if arten:
            eintrag["kind"] = arten[feld]
        if args.delta is not None:
            antwort, fundstelle = entscheidungen.get(feld, (None, None))
            eintrag["decision_on_value"] = {"question": FRAGE, "answer": antwort,
                                            "evidence": fundstelle}
        nachweis["fields"][feld] = eintrag
        probe = f"{i:03d}-" + re.sub(r"[^A-Za-z0-9_.-]", "_", feld)
        # Sicherung, Fingerabdrücke und Rückbau über `gesicherter_datensatz`,
        # denselben Weg wie die Vorproben. Ausdrücklich betreten und verlassen,
        # weil der Rückbau VOR `speichern()` im `finally` liegen muss und der
        # Rumpf dieser Schleife sonst eine Stufe tiefer stünde.
        rahmen = gesicherter_datensatz(eintrag)
        pfade, abdruecke = rahmen.__enter__()
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
            abdruecke["mutated"] = fingerabdruecke(pfade)

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
                # **Feldart und Schrittweite gehören daneben, nicht nur der
                # neue Wert** (AC6): `6000->6010` allein sagt nicht, ob 10
                # überhaupt ankommen kann. Erst „raster, Schritt 10, Takt 10 ms"
                # macht die Frage sichtbar, ohne sie zu beantworten.
                schritt = ", ".join(filter(None, (arten[feld], f"Schritt {args.delta:g}",
                                                  aufloesung(arten[feld], TAKT))))
                print(f"      {proben}   [{schritt}]", flush=True)
                # AC7: die Frage VOR dem Lauf, mit Antwort und Fundstelle. Was
                # hinterher als Erklärung dazukommt, ist keine Prüfung.
                antwort, fundstelle = entscheidungen.get(feld, (None, None))
                print(f"      {FRAGE} "
                      + (f"{antwort} — {fundstelle}" if antwort else
                         "UNBEANTWORTET — ein GRÜN sagt dann nichts über die Deckung"),
                      flush=True)

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
            abdruecke["before_restore"] = fingerabdruecke(pfade)
            veraendert = sorted(f for f, wert in abdruecke["before_restore"].items()
                                if wert != abdruecke["mutated"][f])

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
            # Wer während des Laufs in den Datensatz schreibt, macht jedes Urteil
            # wertlos: Bau und Suiten haben nicht gesehen, was `mutated` sagt.
            verunreinigt = f"Datensatz während des Laufs verändert: {', '.join(veraendert)}"
            if args.nur_bau:
                dauer = time.monotonic() - t0
                stand = "FEHLER" if veraendert else "BAU" if gebrochen else "baut"
                detail = (verunreinigt if veraendert else
                          "Build bricht ab" if gebrochen else "Bau geht durch")
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

            if veraendert:
                stand, detail = "FEHLER", verunreinigt
            elif gebrochen:
                # Im Delta-Modus ist ein unzulässiger SCHRITT schon vor dem Lauf
                # abgewiesen. Bricht der Bau trotzdem, ist der WERT unzulässig
                # (`assert!(amount > 0)` in emit_fixation): weder Pflicht noch
                # ungedeckt.
                stand, detail = "BAU", ("Build bricht ab — der neue Wert ist unzulässig"
                                        if args.delta is not None else "Build bricht ab")
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
            elif args.delta is not None and entscheidungen.get(feld, (None,))[0] != "ja":
                # AC7 von #1087: Ein GRÜN ist nur dann „ungedeckt", wenn eine
                # Entscheidung genau auf dem Wert liegt. Sonst kann die Änderung
                # wirkungslos sein — gemessen an `decayWard.durationMs` +10:
                # Das Fenster wanderte von 12000 auf 12010, der Prüffall bei
                # 12000 blieb drinnen, und das Werkzeug schrieb „UNGEDECKT".
                #
                # Bis #1104 stand hier eine mechanische Prüfung „Schritt neben
                # dem Raster". Seit #1104 lehnt der Bau jeden solchen Schritt
                # an einem Rasterfeld ab; er startet gar nicht mehr
                # (`schritt_unzulaessig`). Übrig bleibt die Frage, die keine
                # Mechanik beantwortet.
                antwort = entscheidungen.get(feld, (None,))[0]
                stand = "GRUEN?"
                detail = ("keine Entscheidung auf dem Wert — GRÜN ohne Aussage" if antwort == "nein"
                          else "Entscheidung auf dem Wert unbeantwortet — GRÜN ohne Aussage")
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
            rahmen.__exit__(None, None, None)
            if not eintrag["restored"]:
                rueckbau_kaputt = feld
            speichern()
    if rueckbau_kaputt:
        raise RuntimeError(f"Rückbau nach {rueckbau_kaputt} nicht byteweise identisch")

    _unterbrechung_pruefen()
    ziel = WURZEL / args.aus
    schreibe(ziel, ergebnisse)
    _unterbrechung_pruefen()
    nachweis["completed"] = len(ergebnisse) == len(felder)
    fehler = any(e.get("status") in ("FEHLER", "ABBRUCH")
                 or any(e.get(sprache, {}).get("status") == "ERROR" for sprache in ("rust", "ts"))
                 for e in nachweis["fields"].values())
    nachweis["full_pass"] = (nachweis["completed"] and not fehler and not args.nur_bau
                              and args.delta is None and not args.feld and not args.ab
                              and not args.art)
    speichern()
    print(f"\n{ziel}")
    gruen = [f for f, s, _, _ in ergebnisse if s == "GRUEN"]
    fraglich = [f for f, s, _, _ in ergebnisse if s == "GRUEN?"]
    print(f"UNGEDECKT: {len(gruen)} von {len(ergebnisse)}")
    for f in gruen:
        print(f"  {f}")
    # **Getrennt gezählt, nicht mitgezählt** (AC6 von #1087). Ein GRÜN ohne
    # „ja" auf die Frage nach der Entscheidung auf dem Wert ist NICHT
    # ungedeckt — es kann eine wirkungslose Änderung sein. Stünde es in
    # derselben Zeile, wiederholte die Wertänderung genau den Fehler, den sie an
    # der Streichung behebt: zwei verschiedene Sachverhalte unter einer Zahl.
    if fraglich:
        print(f"\nOHNE AUSSAGE (GRÜN, aber keine Entscheidung genau auf dem Wert belegt): "
              f"{len(fraglich)}")
        for f in fraglich:
            print(f"  {f}")
        print("  Diese Felder sind WEDER gedeckt NOCH ungedeckt. Erst eine Fundstelle mit"
              "\n  --entscheidung FELD=ja:FUNDSTELLE oder die Gegenrichtung entscheidet.")
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
        if not vergleich["full_pass"]:
            print("KEINE VOLLSTÄNDIGE ABNAHME: Teilmenge, Nur-Bau, Wertänderung, "
                  "unvollständige Suite oder ohne bestandene Vorproben.")
    return 1 if fehler else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except NichtGestartet as grund:
        print(f"NICHT GESTARTET: {grund}", file=sys.stderr)
        sys.exit(2)
    except NichtGelaufen as grund:
        # Exit 2 wie die übrigen „nicht gemessen"-Ausgänge dieses Werkzeugs.
        # Das betroffene Feld steht im Nachweis als ABBRUCH, sein Datensatz ist
        # zurückgestellt (`finally` der Feldschleife).
        print(f"NICHT GELAUFEN: {grund}", file=sys.stderr)
        print("  Kein Feld wurde deswegen verbucht. Erst `cargo remote --status` ansehen,"
              " dann mit --ab <feld> fortsetzen.", file=sys.stderr)
        sys.exit(2)
