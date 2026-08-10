---
id: groessen-abdeckung
status: Stable
ssot: true
refs:
  docs: [GAME-DESIGN.md]
affects: [tests/unit, sim/tests, tests/parity]
---

# Welche Größen im Zauber-Bestand geprüft sind — und welche nicht

Zuletzt vollständig gemessen am **2026-08-10** (alle 109 Paare), Werkzeug
`tools/groessen-mutation.py`. Erstmals erhoben am 2026-08-07 für Issue #547;
die datierten Nachträge am Ende halten die Zwischenstände fest.

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

**109 Paare** über 37 Felder und 35 Zauber, alle gemessen. Der Bestand ist
seit der ersten Messung gewachsen: 53 vor #540, 68 mit den fünf Verstärkern,
90 mit den Textfeldern aus #633, 109 mit den fünf COAGULATION-Zaubern aus
#481.

---

## Das Ergebnis: 14 Lücken von 109

| Sorte            | Paare | Was anschlägt                                                   |
| ---------------- | ----- | --------------------------------------------------------------- |
| **geschützt**    | 83    | mindestens ein Wirkungs-Fall                                    |
| **nur Frische**  | 10    | allein die Aufzeichnungs-Prüfung — ein Neu-Export beruhigt sie  |
| **schweigt**     | 4     | nichts                                                          |
| **strukturell**  | 6     | die Laufzeit-Prüfung, weil es keinen zweiten gültigen Wert gibt |
| **rust-geprüft** | 6     | der Rust-Bau; vitest allein schweigt (siehe Kasten unten)       |

**10 + 4 = 14 Paare melden keine Wirkungsprüfung.** Die sechs strukturellen
und die sechs rust-geprüften sind keine Lücken: Bei den einen erzwingt der
Typ den einzigen gültigen Wert, bei den anderen prüft die Rust-Seite, was
vitest nicht sieht.

Diese Unterscheidung gehört in die Überschrift und nicht nur in den Text. Eine
Zahl, die Nicht-Lücken als Lücken führt, fordert zu Arbeit auf, die niemand
tun kann — derselbe Fehler wie eine Rangfolge, die Ungemessenes als Null
ausweist.

### Nachzug 2026-08-10: zwölf Lücken geschlossen

`distil`, `flashpoint` und `virulence` hatten je **alle vier** Buff-Felder
ohne Wirkungsprüfung — zwölf der 26 Lücken, alle demselben Muster aus #540
folgend. `tests/unit/buff-groessen-540.test.ts` schließt sie in einem Zug:

```text
  flashpoint  resource   KINDLEs Hitze-Gewinn         3 -> 6
  distil      cdreduce   WISPs Verkürzung an PYROCLASM  500 -> 1000
  virulence   stack      FESTERs Schaden je Stapel     95 -> 114
```

Je Zauber vier Zusicherungen: der Faktor verdoppelt, der Zwilling **ersetzt**
statt zu stapeln, das Fenster endet — und die **Kreuzprobe**, dass der Faktor
die beiden FREMDEN Größen unberührt lässt. Ohne sie fiele ein vertauschtes
`target` nicht auf: Der Faktor wirkte, nur woanders.

Gegengeprüft: Jede der zwölf Mutationen macht mindestens einen Fall rot.

### `buff.kind` ist NICHT mehr unprüfbar

Bis zum Lauf vom 2026-08-10 stand hier, das Feld sei „heute unprüfbar, weil
die Lage fehlt, in der die Größe entscheidet". Das ist widerlegt — von zwei
Trägern:

```text
buff.kind  griplock     geschuetzt [1 Prueffall]  (buff-ersetzung-ueber-zauber.test.ts)
buff.kind  slag         geschuetzt [4 Prueffaelle] (buff-expire-log, buff-stapelung-ueber-zauber)
```

Die Lage gibt es also. Sie fehlte nur bei vier der sieben Träger — und das war
eine Lücke, keine Unmöglichkeit. Der Unterschied entscheidet darüber, ob
jemand die Arbeit anfängt: Drei der vier (`distil`, `flashpoint`, `virulence`)
sind mit dem Nachzug oben geschlossen, `quicksilver` steht noch offen.

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

Heute ist keine der 109 Zahlen falsch, und beide Seiten rechnen sie gleich. Der
Befund betrifft die Werte, die **morgen** geändert werden — und der
Balance-Läufer (#118) ist genau dafür gebaut. Wenn er anfängt, Zahlen
vorzuschlagen, ist bei **14 von 109** kein Netz da.

---

## Die Schweiger: vier nach dem Nachzug vom 2026-08-10

Die Abschnitte unter dieser Überschrift stammen aus den Läufen vom 2026-08-06
und 2026-08-07 und beschreiben die damaligen zwölf. **Fünf von ihnen sind
inzwischen geschützt** — der volle Lauf vom 2026-08-10 hat es gemessen. Die
Begründungen bleiben stehen, weil sie die SORTE Lücke erklären; welche Paare
heute noch offen sind, sagt die Rohdaten-Datei, nicht dieser Text.

### Sechs gehören zu #547

| Feld                     | Zauber    | Mutation    |
| ------------------------ | --------- | ----------- |
| `control.amountMs`       | stall     | 2000 → 1000 |
| `control.amountMs`       | latch     | 4000 → 2000 |
| `control.cost`           | capstone  | 8 → 4       |
| `control.cost`           | wedge     | 3 → 1       |
| `shield.durationMs`      | scab      | 8000 → 4000 |
| `spender.bonusThreshold` | pyroclasm | 15 → 7      |

### Fünf sind heute unprüfbar — und zählen NICHT als Lücke

`buff.kind` bei fünf seiner sechs Träger: slag, flashpoint, quicksilver,
distil, virulence.

> **`griplock` ist seit dem 2026-08-07 heraus.** Worker-2s Fall in #555 stellt
> die Lage her, die dem Feld dort gefehlt hat. Nachgemessen mit dem
> Mutations-Werkzeug, nicht übernommen:
>
> ```text
> buff.kind  griplock  0 -> 1  geschuetzt [1 Prueffall]
>                              (buff-ersetzung-ueber-zauber.test.ts)
> ```
>
> Genau **ein** tragender Fall — dieselbe dünne Stelle wie
> `quicksilver.buff.target`. Fällt er weg, ist das Feld wieder dort, wo es
> vorher stand, und diese Tabelle ist wieder richtig, ohne dass jemand sie
> anfasst.

`kind` ist der Platz **innerhalb** eines Verstärkungs-Speichers; zwei
Verstärker mit gleichem `target` **und** gleichem `kind` verdrängen einander.
Angeschlossen ist es auf beiden Seiten (`combat.ts` → `applyForGcds`,
`fight.rs` → `apply_for_gcds`, `is_active`). Nur hat heute **kein Speicher
zwei echte Bewohner** — die Zwillinge sind durchweg Prüf-Zauber auf Stufe 101.

Ein Platz, um den sich niemand streitet, kann nicht falsch vergeben werden.
Die Mutation bricht nichts, weil die Lage fehlt, in der `kind` entscheidet —
nicht, weil eine Prüfung fehlt. Was fehlt, ist ein **zweiter echter Verstärker
im selben Speicher**.

> **Dieser Absatz galt bis zum 2026-08-10 und gilt nicht mehr allgemein.**
> `griplock` und `slag` haben inzwischen tragende Fälle: Die Lage lässt sich
> herstellen. Sie fehlt nur noch bei vier Trägern — und damit ist es eine
> Lücke, keine Unmöglichkeit.
>
> Die alte Fassung zählte `buff.kind` deshalb aus den Lücken heraus. Das war
> zum damaligen Stand richtig und ist heute falsch; eine Ausnahme, die sich
> selbst überlebt, hält Arbeit auf, die getan werden kann.

> Eine frühere Fassung führte hier „einen" Schweiger unter #555, mit der
> Begründung „einziger Träger". Beides war zu grob: Es sind sechs Träger, und
> entscheidend ist nicht ihre Zahl, sondern dass jeder allein in _seinem_
> Speicher sitzt.

> **Dieser Kasten stand hier bis zum 2026-08-10 und ist eingelöst.** Er sagte:
> „Die Kopfzahlen (39 Lücken, von 68) stammen aus dem Lauf vom 2026-08-06 und
> sind NICHT nachgezogen … Für die neuen Zahlen bräuchte es einen
> vollständigen Neulauf; er ist nicht gefahren."
>
> Der Lauf ist gefahren: alle **109** Paare, in zwei Blöcken, rund achtzig
> Minuten. Die Kopfzahlen oben stammen daraus.
>
> **Eine Auflage für den nächsten Lauf.** Das Werkzeug fährt NUR vitest — das
> steht in seinem eigenen Kopf. Die sechs Zeilen mit `RUST-GEPRUEFT` hat
> jemand von Hand ergänzt, nachdem er den Rust-Bau gegengeprüft hatte; ein
> neuer Lauf kann sie nicht kennen und setzt sie zwangsläufig auf „NUR
> FRISCHE" oder „SCHWEIGT" zurück.
>
> Wer die Rohausgabe ungeprüft in `groessen-abdeckung-messung.txt` schreibt,
> **löscht diese Erkenntnisse** und behauptet sechs Lücken, die es nicht gibt.
> Gemessen am 2026-08-10: vier Zeilen `aether`, zwei `pyroclasm`.

### Warum die sechs echten Lücken verschieden teuer sind

Gemessen wurde für jeden Schweiger, ob sein Zauber in einer Aufzeichnung
überhaupt **gedrückt** wird:

| Sorte                  | Paare                                                                                               | Was fehlt                                                               | Aufwand                                            |
| ---------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------- |
| ~~keine Aufzeichnung~~ | ~~control.amountMs/stall, shield.durationMs/scab~~                                                  | **Widerlegt am 2026-08-07** — siehe unten.                              | —                                                  |
| **keine Lage**         | control.amountMs/latch, control.cost/capstone, control.cost/wedge, spender.bonusThreshold/pyroclasm | Der Zauber **wird** gewirkt, aber sein Betrag ändert am Ausgang nichts. | eine Aufzeichnung, die die **Situation** herstellt |
| **heute unprüfbar**    | buff.kind, fünf der sechs Träger (nicht griplock)                                                   | Die Lage, in der die Größe entscheidet, gibt es im Bestand nicht.       | nichts — siehe oben und #555                       |

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
python3 tools/groessen-mutation.py 0 40     # Paare 0 bis 39
python3 tools/groessen-mutation.py 40 109   # der Rest
```

In Blöcken, weil ein Lauf über alle 109 Paare rund achtzig Minuten braucht —
gemessen am 2026-08-10: zwanzig Minuten für vierzig Paare, knapp eine Stunde
für die übrigen 69.

**Die Ausgabe gehört nicht ungeprüft in die Datei.** Ein Lauf kennt die
`RUST-GEPRUEFT`-Vermerke nicht und setzt sie zurück; siehe den Kasten weiter
oben.

**Die Zusicherung darin ist kein Beiwerk:** Vor jeder Messung prüft das
Skript, dass die Mutation die Datei _wirklich_ verändert hat. Ohne sie sieht
ein Feld, dessen Mutation nicht griff, exakt aus wie ein ungeprüftes — beide
schweigen. Am 2026-08-06 ist genau das passiert: Ein Suchmuster passte nach
einer Formatierung nicht mehr, der Aufruf lief fehlerfrei durch, und beide
„Läufe" maßen denselben Code. Gemeldet wurde daraufhin eine Lücke, die keine
war.

---

## Wie schwer wöge ein Fehler? Der Schadenshebel

Für die Rangfolge aus AC3 — welche Lücke zuerst? — je Größe gemessen: Wert
halbieren, exportieren, `expected.damage` aller Aufzeichnungen vergleichen.
Rohdaten: [`groessen-hebel-messung.txt`](groessen-hebel-messung.txt).

| Hebel   | Größe                     | Träger    | Aufzeichnung    |
| ------- | ------------------------- | --------- | --------------- |
| 72,58 % | `dot.tickIntervalMs`      | searing   | new-calcination |
| 48,13 % | `dot.tickIntervalMs`      | cinder    | cinder-ticks    |
| 33,87 % | `dot.tickDamage`          | searing   | new-calcination |
| 23,26 % | `dot.tickDamage`          | cinder    | cinder-ticks    |
| 21,53 % | `dot.tickDamage`          | rot       | rot-decay       |
| 14,44 % | `dot.durationMs`          | cinder    | cinder-ticks    |
| 10,48 % | `spender.damagePerPoint`  | pyroclasm | dot-lifetime    |
| 10,48 % | `spender.bonusMultiplier` | pyroclasm | dot-lifetime    |
| 1,15 %  | `convert.perSecond`       | efflux    | efflux-restzeit |
| 0,14 %  | `buff.gcds`               | virulence | buff-targets    |

### Die übrigen 35 stehen auf 0,00 % — aus drei verschiedenen Gründen

Und nur zwei davon sind Befunde.

| Grund                                                    | Fälle                                                          | Was er bedeutet                              |
| -------------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------- |
| **(a) Träger wird nie gewirkt**                          | anchor, scab, stall, stay                                      | Befund: keine Aufzeichnung wirkt ihn         |
| **(b) Träger macht gar keinen Schaden**                  | salve, plate, temper, die fünf Verstärker, die Kontroll-Zauber | **kein Befund — die Messgröße greift nicht** |
| **(c) Träger macht Schaden, die Größe ändert ihn nicht** | Rest                                                           | Befund: die Lage fehlt                       |

**(b) ist eine Grenze dieser Messung, kein Ergebnis.** Eine Heilung, die keinen
Schaden bewegt, ist kein Fund — das ist die Definition einer Heilung. Für
Schilde bräuchte es `absorbed`, für `salve` `healed`, für Kontrolle
`damage_taken`. Alle drei stehen im `RunResult`, aber die Aufzeichnungen
führen nur `expected.damage`.

> Derselbe Fehler wie „1 rot heißt geschützt", eine Ebene weiter: eine Zahl
> genommen, weil sie leicht zu holen war, ohne ihre Reichweite zu prüfen.

### Was der Hebel NICHT misst

Eine 0 heißt „in dieser Aufzeichnung", nicht „diese Größe". Gemessen am
Gegenbeispiel:

```
Basislauf first-crucible      4240,0
distil buff.factor 2 → 1      4240,0    exakt gleich
```

Nicht weil `distil` nichts bewegt — die Standard-APL wirkt ihn nicht. Wo der
Träger nicht wirksam vorkommt, misst das Verfahren die **Aufzeichnung** statt
die Größe. Die zehn oben sind davon nicht betroffen; für die übrigen bräuchte
es je Träger eine Rotation, die ihn wirkt.

**Die Gegenprobe, dass das Verfahren überhaupt misst:** `kindle baseDamage
100 → 50` bewegt die Schadenssumme über 48 Aufzeichnungen von 95562 auf 79749
— 16,55 %. Ohne diesen Beleg wären 35 Nullen nicht von einem kaputten
Werkzeug zu unterscheiden.

---

## Nachtrag 2026-08-07: die Sorte „keine Aufzeichnung" gibt es nicht

Oben stand, `stall`, `scab`, `anchor` und `stay` kämen in **keiner**
Aufzeichnung vor. Das war falsch, und der Fehler lag im Messwerkzeug, nicht in
den Daten:

```
fixation.json runs[1]:  Slots gedrückt [0, 1, 2, 3]
  Slot 0 = bind    Slot 1 = stay    Slot 2 = anchor    Slot 3 = stall
```

Zwei Gründe, beide dieselbe Klasse:

1. **Zwei Press-Formen.** Der Bestand führt `{slot, tick}` **und**
   `{spellId, tick}`. Die erste Messung las nur eine.
2. **Bindungen je Lauf.** Bei der `runs`-Bauart steht `bindings` in jedem
   einzelnen Lauf, nicht auf Dateiebene — dort liefert es `None`. Wer nur die
   Dateiebene liest, findet keine Zuordnung und meldet null Drücke.

**Damit sind alle sechs Lücken der Sorte „keine Lage".** Der Zauber wirkt, die
Größe ändert nichts — es fehlt die Situation, nicht die Aufzeichnung. Das ist
teurer zu beheben als gedacht: Es braucht keine neue Belegung, sondern einen
Kampf, in dem der Betrag den Ausgang ändert.

> Ein `.get('slot')` statt `p['slot']` hätte hier still `None` geliefert und
> denselben falschen Befund erzeugt — ohne Fehlermeldung. Der Abbruch bei
> unbekannter Form ist deshalb Absicht: Er ist die einzige Stelle, an der eine
> dritte Bauart überhaupt auffällt.

---

## Drei Fallen in dieser Datei-Sammlung — lies das, bevor du misst

An einem einzigen Tag (2026-08-07) haben diese drei zusammen **vier
Fehlmessungen** erzeugt, zwei bei jedem von zwei Messenden. Jede sah aus wie
ein Befund, keine wurde von einem Wächter gefunden.

### 1. Drei Aufzeichnungs-Bauarten

```
presses + expected   ein Lauf je Datei
runs                 mehrere Läufe je Datei   (coagulation, ward-cap, fixation, player-damage)
anders               kein Kampflauf           (buffs.json führt `steps`)
```

Wer „über alle Aufzeichnungen" misst, misst je nach Werkzeug **39, 43, 47
oder 52** — und jede Zahl ist für eine andere Frage richtig:

| Zahl | beantwortet                                                             |
| ---- | ----------------------------------------------------------------------- |
| 52   | Wie viele Dateien liegen im Verzeichnis?                                |
| 43   | Wie viele davon sind abspielbar?                                        |
| 38   | Wie viele führen mindestens eine Wirkungs-Größe?                        |
| 47   | Wie viele **Läufe** sind das? (34 einzeln + 13 aus vier `runs`-Dateien) |

_Erzeugte Fehlmessung:_ „In keiner Aufzeichnung wird geheilt" — gemessen
waren 39 von 52, und `coagulation#2` heilt 3000.

### 2. Zwei Press-Formen

`{slot, tick}` **und** `{spellId, tick}`. Ein Werkzeug, das nur eine kennt,
findet die Zauber der anderen nicht.

_Erzeugte Fehlmessung:_ „`anchor`, `stall`, `stay` werden nie gedrückt."

### 3. Bindungen stehen je LAUF, nicht auf Dateiebene

Bei der `runs`-Bauart liegt `bindings` in jedem einzelnen Lauf; auf
Dateiebene liefert es `None`. Wer dort nachsieht, findet keine Zuordnung von
Slot zu Zauber — und meldet null Drücke.

_Erzeugte Fehlmessung:_ dieselbe wie oben, aus dem anderen Grund. Zwei Wege
zum selben falschen Ergebnis.

### Was daraus folgt

**Ein `.get()` ist hier gefährlicher als ein `KeyError`.** Ein fehlender
Schlüssel liefert still `None`, der Zauber gilt als „nie gewirkt", und das
Werkzeug meldet einen Befund statt eines Fehlers. Der Abbruch bei unbekannter
Form ist die einzige Stelle, an der eine vierte Bauart überhaupt auffiele.

**Und die Gegenprobe, die alle vier Fehlmessungen gefunden hat, war dieselbe:**
Das Ergebnis war zu gleichförmig. Zehn von zehn Zaubern nie gedrückt, sechs von
sechs Größen „SCHWEIGT", 39 von 39 Aufzeichnungen ohne Heilung. Eine Liste, in
der alle Zeilen gleich aussehen, ist verdächtig — auch wenn das Ergebnis zur
Erwartung passt.

---

## Nachtrag 2026-08-07 (#592): die zehn teuersten Lücken sind geschlossen

Aus **39 Lücken** werden **29**. Geschlossen wurde nach Hebel, nicht
alphabetisch — wer bei 72,58 % anfängt, sichert mit drei Fällen mehr ab als mit
zehn kleinen.

| Hebel   | Größe                     | Träger      | Prüffall                          |
| ------- | ------------------------- | ----------- | --------------------------------- |
| 72,58 % | `dot.tickIntervalMs`      | searing     | `dot-groessen.test.ts`            |
| 56,00 % | `conversion.sharePercent` | temper      | `umwandlung-groessen.test.ts`     |
| 50,00 % | `heal.amount`             | salve       | `heilung-schild-groessen.test.ts` |
| 50,00 % | `conversion.durationMs`   | temper      | `umwandlung-groessen.test.ts`     |
| 48,13 % | `dot.tickIntervalMs`      | cinder      | `dot-groessen.test.ts`            |
| 33,87 % | `dot.tickDamage`          | searing     | `dot-groessen.test.ts`            |
| 33,33 % | `shield.amount`           | scab        | `heilung-schild-groessen.test.ts` |
| 24,11 % | `buff.gcds`               | quicksilver | `umwandlung-groessen.test.ts`     |
| 24,11 % | `buff.factor`             | quicksilver | `umwandlung-groessen.test.ts`     |
| 23,26 % | `dot.tickDamage`          | cinder      | `dot-groessen.test.ts`            |

**Der Nachweis ist das Werkzeug, nicht die Behauptung.**
`tools/groessen-mutation.py` meldet für alle zehn jetzt `geschuetzt (…)` statt
`NUR FRISCHE`.

### Was dabei zweimal fast schiefging

Beide Male hätte der Prüffall **grün** dagestanden und nichts geprüft:

- **SALVE bei vollem Leben heilt null.** Gegen den Standard-Boss fällt in zehn
  Sekunden kein Schaden, gegen `test-caster` genau 96 — und die Heilung deckte
  96 statt 1500 ab, weil mehr nicht fehlte. Bei `heal.amount 1500 → 750` wäre
  der Fall grün geblieben. Deshalb steht als eigener Fall daneben: Es müssen
  mindestens 1500 fehlen.
- **TEMPER wandelt AUSGETEILTEN Schaden um**, nicht erlittenen. Ein erster
  Entwurf ließ den Spieler 1600 Schaden nehmen und bekam `converted: 0`.

Beides ist dieselbe Klasse wie die Standardoptionen aus #576: **Ohne die
richtige Lage misst ein Fall nichts und ist trotzdem grün.**

---

## Nachtrag 2026-08-10 (#481): der erste vollständige Neulauf

Der Kasten weiter oben verlangte ihn seit dem 2026-08-06 („er ist nicht
gefahren"). Jetzt ist er gefahren — **109 von 109 Paaren**, in zwei Blöcken,
rund achtzig Minuten.

|              | vorher (fortgeschrieben) | jetzt (gemessen) |
| ------------ | ------------------------ | ---------------- |
| geschützt    | 47                       | **71**           |
| nur Frische  | 39                       | **19**           |
| schweigt     | 11                       | **7**            |
| strukturell  | 6                        | 6                |
| rust-geprüft | 6                        | 6                |

**24 Paare sind besser geworden, 4 schlechter.** Die vier waren CRUCIBLE SKIN
(#481) — und sie sind der Grund, warum dieser Lauf sich gelohnt hat.

### Was die vier gezeigt haben

`crucible-skin.test.ts` prüfte RICHTUNG statt GRÖSSE:

```ts
expect(mit.absorbiert).toBeGreaterThan(ohne.absorbiert);
```

Halbiert man `shield.amount`, fängt die Schicht immer noch mehr ab als gar
keine. Grün. Die vier Prüffälle hinter dem alten Vermerk „geschuetzt [4]"
waren `coverage`, `jede-groesse-ist-gemessen`, `sound-triggers` und die
Frische — **Listen und Frische, nicht Wirkung.** Sie werden rot, weil die
Aufzeichnung nicht mehr zur Datei passt; wer sie neu erzeugt, ist die Meldung
los.

> „Geschützt" hieß dort _irgendetwas wird rot_, nicht _die Größe ist geprüft_.

### Und was die Behebung gezeigt hat

Der erste Nachtrag zum Prüffall färbte bei jeder Halbierung — aber nur über
abgeschriebene Datensatz-Zeilen (`expect(zauber!.shield!.amount).toBe(1800)`).
Ohne sie blieb er bei allen vier Halbierungen grün.

Die Ursache war der **Träger**, nicht die Zusicherung: `test-warden` teilt 720
aus, da hat keine der drei Größen Platz. Mit `test-onslaught` (1600 je
Treffer) und KINDLE als Füller fängt der Wirkungsteil allein alle fünf
Halbierungen.

Damit steht diese Lehre zum dritten Mal in diesem Dokument, jedes Mal aus
einer anderen Richtung: **Eine Probe braucht einen Träger, bei dem die Wirkung
Platz hat.**
