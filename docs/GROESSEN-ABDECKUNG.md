---
id: groessen-abdeckung
status: Stable
ssot: true
refs:
  docs: [GAME-DESIGN.md]
affects: [tests/unit, sim/tests, tests/parity]
---

# Welche Größen im Zauber-Bestand geprüft sind — und welche nicht

Gemessen am 2026-08-07 für Issue #547, Werkzeug `tools/groessen-mutation.py`.

**Die Frage.** Ein Prüffall, der nur die _Existenz_ einer Wirkung feststellt
(`toBeGreaterThan(CAP)`), bleibt grün, wenn jemand ihre _Größe_ auf ein
Zehntel senkt. SLAG ließ sich so von 50 % Anhebung auf 5 % bringen, ohne dass
sich eine der 1764 Prüfungen meldete.

**Das Verfahren.** Je Wert: halbieren, die **volle** Suite fahren, rote Fälle
zählen, zurückstellen. Nicht die naheliegende Testdatei — „ungeprüft" heißt,
dass _keine_ Prüfung anschlägt, und ein Feld kann über eine
Paritäts-Aufzeichnung abgesichert sein, ohne dass eine Datei seinen Namen
trägt.

---

## Die Messeinheit ist das Paar (Zauber, Feld)

Der erste Durchgang maß je Feld einen Träger — 24 Läufe, zwei Schweiger. Das
Ergebnis war falsch:

```
control.amountMs   stall     2000 -> 1000    SCHWEIGT
control.amountMs   stay      3000 -> 1500    1 rot
control.amountMs   capstone  2000 -> 1000    1 rot
control.amountMs   latch     4000 -> 2000    SCHWEIGT
```

Zwei von vier Trägern desselben Feldes sind ungeschützt, zwei nicht. Eine
Aussage auf Feld-Ebene ist in **beide** Richtungen falsch: Ein abgesicherter
erster Träger verdeckt die Lücke dahinter, ein ungeschützter erklärt Zauber
für ungeprüft, die es nicht sind.

**68 Paare** über 24 Felder, alle gemessen (53 vor #540, dann kamen die fünf
Verstärker mit ihren drei `buff`-Größen dazu).

---

## Das Ergebnis: 45 von 68 sind faktisch ungeschützt

| Sorte           | Paare | Was anschlägt                                                  |
| --------------- | ----- | -------------------------------------------------------------- |
| **nur Frische** | 33    | allein die Aufzeichnungs-Prüfung — ein Neu-Export beruhigt sie |
| **schweigt**    | 12    | nichts; davon 6× `buff.kind` (siehe „heute unprüfbar")         |
| **geschützt**   | 23    | mindestens ein Wirkungs-Fall                                   |

Rohdaten je Paar mit gefahrener Mutation und roten Dateien:
[`groessen-abdeckung-messung.txt`](groessen-abdeckung-messung.txt).

### Die erste Antwort war „sieben von 53" — und sie war falsch

Nicht weil schlechter gemessen worden wäre: Die Zahlen stimmten. Falsch war
die **Deutung**. „1 rot" hieß geschützt — und in 33 Fällen ist dieses eine Rot
`freshness.test.ts`.

Eine Frische-Prüfung meldet, dass die eingecheckte Aufzeichnung nicht mehr zum
Code passt. Gegen eine _unbeabsichtigte_ Änderung ist das genau richtig. Gegen
eine _absichtliche_ Balance-Änderung ist es wirkungslos, weil
`npm run parity:export` zum Arbeitsablauf gehört und keine Rückfrage stellt.

Gemessen, beide Richtungen, mit `groessen-export-probe.py`:

```
dot.durationMs:searing    vor 3 Dateien / 1 Rust → nach 2 / 0   GESCHÜTZT
convert.perSecond:efflux  vor 1 Datei   / 1 Rust → nach 0 / 0   nur Aufzeichnung
```

**Die Bitgleichheit sichert, dass beide Seiten dasselbe rechnen — nicht, dass
sie das Richtige rechnen.**

### Wann das zuschlägt

Heute ist keine der 68 Zahlen falsch, und beide Seiten rechnen sie gleich. Der
Befund betrifft die Werte, die **morgen** geändert werden — und der
Balance-Läufer (#118) ist genau dafür gebaut. Wenn er anfängt, Zahlen
vorzuschlagen, ist bei 45 von 68 kein Netz da.

---

## Die zwölf Schweiger

### Sechs gehören zu #547

| Feld                     | Zauber    | Mutation    |
| ------------------------ | --------- | ----------- |
| `control.amountMs`       | stall     | 2000 → 1000 |
| `control.amountMs`       | latch     | 4000 → 2000 |
| `control.cost`           | capstone  | 8 → 4       |
| `control.cost`           | wedge     | 3 → 1       |
| `shield.durationMs`      | scab      | 8000 → 4000 |
| `spender.bonusThreshold` | pyroclasm | 15 → 7      |

### Sechs sind heute unprüfbar — und zählen NICHT als Lücke

`buff.kind` bei allen sechs Trägern (slag, flashpoint, quicksilver, distil,
griplock, virulence).

`kind` ist der Platz **innerhalb** eines Verstärkungs-Speichers; zwei
Verstärker mit gleichem `target` **und** gleichem `kind` verdrängen einander.
Angeschlossen ist es auf beiden Seiten (`combat.ts` → `applyForGcds`,
`fight.rs` → `apply_for_gcds`, `is_active`). Nur hat heute **kein Speicher
zwei echte Bewohner** — die Zwillinge sind durchweg Prüf-Zauber auf Stufe 101.

Ein Platz, um den sich niemand streitet, kann nicht falsch vergeben werden.
Die Mutation bricht nichts, weil die Lage fehlt, in der `kind` entscheidet —
nicht, weil eine Prüfung fehlt. Was fehlt, ist ein **zweiter echter Verstärker
im selben Speicher**.

> Diese Sorte darf nicht als Lücke gezählt werden. Sonst steht hier eine Zahl,
> die zu Arbeit auffordert, die niemand tun kann.

> Eine frühere Fassung führte hier „einen" Schweiger unter #555, mit der
> Begründung „einziger Träger". Beides war zu grob: Es sind sechs Träger, und
> entscheidend ist nicht ihre Zahl, sondern dass jeder allein in _seinem_
> Speicher sitzt.

### Warum die sechs echten Lücken verschieden teuer sind

Gemessen wurde für jeden Schweiger, ob sein Zauber in einer Aufzeichnung
überhaupt **gedrückt** wird:

| Sorte                  | Paare                                                                                               | Was fehlt                                                               | Aufwand                                            |
| ---------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------- |
| **keine Aufzeichnung** | control.amountMs/stall, shield.durationMs/scab                                                      | Der Zauber wird in keiner Aufzeichnung gewirkt.                         | eine Aufzeichnung                                  |
| **keine Lage**         | control.amountMs/latch, control.cost/capstone, control.cost/wedge, spender.bonusThreshold/pyroclasm | Der Zauber **wird** gewirkt, aber sein Betrag ändert am Ausgang nichts. | eine Aufzeichnung, die die **Situation** herstellt |
| **heute unprüfbar**    | buff.kind, alle sechs Träger                                                                        | Die Lage, in der die Größe entscheidet, gibt es im Bestand nicht.       | nichts — siehe oben und #555                       |

Die Belege für die zweite Sorte:

```
dot-lifetime.json      pyroclasm       gedrückt, bonusThreshold schweigt
fixation-spells.json   capstone wedge  gedrückt, cost schweigt
kern-zauber.json       latch           gedrückt, amountMs schweigt
```

**Die mittlere Sorte ist die teure.** Ein Prüffall an der Zahl behebt sie
nicht: `slowCast` wirkt nur, während der Boss wirkt, `delayStrike` nur, wenn
der verschobene Schlag den Verlauf ändert. Fehlt diese Lage, ist der Betrag
korrekt gelesen und ohne Folge — und jede Prüfung sieht ihn als richtig an.

Das ist heute die dritte Ausprägung derselben Klasse: `buff.kind` (kein
zweiter Bewohner im selben Speicher), `phases` (kein Ereignis hängt daran,
#557) und diese vier.
Dreimal steht der Code da, wird der Wert gelesen, und die Situation, in der er
einen Unterschied macht, kommt nicht vor.

---

## Das Muster: die Absicherung hängt am ZAUBER, nicht am Feld

Bei jedem Feld mit mehreren Trägern ist **genau einer** laut abgesichert, die
übrigen sind leise oder still:

| Feld                      | laut            | leise oder still           |
| ------------------------- | --------------- | -------------------------- |
| `spender.damagePerPoint`  | pyre **41**     | pyroclasm 1                |
| `spender.bonusThreshold`  | pyre **36**     | pyroclasm **0**            |
| `spender.bonusMultiplier` | pyre **34**     | pyroclasm 1                |
| `shield.amount`           | crust **5**     | plate 1, scab 1            |
| `shield.durationMs`       | crust **3**     | plate 1, scab **0**        |
| `dot.tickIntervalMs`      | smoulder **50** | cinder 1, searing 1, rot 5 |
| `dot.tickDamage`          | smoulder **35** | cinder 1, searing 1, rot 1 |
| `resourceGain.heat`       | kindle **48**   | bellows 3, ember 1         |

Der laute Träger ist der, der in den Paritäts-Aufzeichnungen vorkommt. Wer
nach ihm einen zweiten Zauber mit demselben Feld baut, erbt dessen Absicherung
**nicht** — er bekommt sie nur, wenn er selbst in eine Aufzeichnung kommt.

**Das ist die eigentliche Auskunft dieser Messung.** Sie erklärt auch, warum
die Lücken dort sitzen, wo sie sitzen: nicht in einer bestimmten Schule,
sondern beim jeweils _späteren_ Träger.

Die naheliegende Deutung — „den Kontroll-Zaubern fehlt eine Aufzeichnung" —
war die erste Vermutung nach 26 von 53 Paaren, als alle Schweiger noch in der
FIXATION lagen. Die letzten 14 Paare haben sie widerlegt: `scab`
(COAGULATION) und `pyroclasm` (CALCINATION) schweigen ebenso.

---

## Die Regel für jeden Größen-Prüffall

**Ein Sollwert aus derselben Quelle wie der Istwert ist kein Sollwert.**

Der erste Entwurf des SLAG-Falls rechnete `round(CAP * slag.buff.factor)` und
wäre bei der Gegenprobe **grün** geblieben: erwartet `round(20 × 1.05) = 21`,
gemessen 21 — die Formel gegen sich selbst. Die Sollwerte stehen deshalb hart
im Prüffall, mit Herleitung im Kommentar. Wer sie absichtlich ändert, ändert
diese Zeile mit, und genau dann fällt auf, dass eine Entscheidung getroffen
wurde.

## Und ein Feld kann mehrere Größen tragen

`buff` trägt einen Betrag (`factor`) und eine Dauer (`gcds`), `dot` deren
drei. Eine Zeile „`dot.tickDamage` ist geschützt" lässt offen, ob
`tickIntervalMs` es auch ist — deshalb steht in den Tabellen oben je Zeile die
**gefahrene Mutation**, nicht nur das Urteil.

---

## Das Werkzeug

```bash
python3 tools/groessen-mutation.py 0 13     # Paare 0 bis 12
python3 tools/groessen-mutation.py 39 53    # der Rest
```

In Blöcken, weil ein Lauf über alle 53 Paare rund vierzig Minuten braucht.

**Die Zusicherung darin ist kein Beiwerk:** Vor jeder Messung prüft das
Skript, dass die Mutation die Datei _wirklich_ verändert hat. Ohne sie sieht
ein Feld, dessen Mutation nicht griff, exakt aus wie ein ungeprüftes — beide
schweigen. Am 2026-08-06 ist genau das passiert: Ein Suchmuster passte nach
einer Formatierung nicht mehr, der Aufruf lief fehlerfrei durch, und beide
„Läufe" maßen denselben Code. Gemeldet wurde daraufhin eine Lücke, die keine
war.
