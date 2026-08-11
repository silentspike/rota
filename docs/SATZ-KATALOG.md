---
id: satz-katalog
status: Draft
ssot: false
refs:
  docs: [GAME-DESIGN.md, AUSRUESTUNGS-RASTER.md]
affects: [src/gear/sets.ts, data/items.json]
---

# Der Satz-Katalog — Vorschlag

> **Das ist ein Vorschlag, keine Entscheidung.** Der Auftraggeber hat am 2026-08-11
> gewählt: „Ich schlage sie vor, du entscheidest". Nichts hier ist gebaut, und nichts
> gilt, bevor er es bestätigt. Der Plan (`~/.claude/plans/greedy-skipping-russell.md`)
> ist die Grundlage, nicht meine Erfindung.

## Was der Plan vorgibt — und was daraus folgt

| Vorgabe (Plan)                                                                       | Folge für diesen Katalog                           |
| ------------------------------------------------------------------------------------ | -------------------------------------------------- |
| **6 Endgame-Instanzen à 6 Bosse**, ein Satz je Instanz                               | genau sechs Sätze                                  |
| **Jeder Satz zielt auf eine Schule** — „auch ungenutzte Zauber bekommen ihre Nische" | fünf Schulen, also **einer bleibt übrig**          |
| Drei Stufen: **2 / 4 / 6 Teile**                                                     | `SET_TIERS` steht, ist nicht vorläufig             |
| **Werte unten, Rotations-Effekte oben**                                              | Stufe 2 und 4 tragen Anteile, Stufe 6 einen Effekt |
| **Frei kombinierbar**, nur die Anzahl zählt                                          | 4 von A plus 2 von B muss sich lohnen              |
| Satz-Boni **addieren**, Affixe **multiplizieren**                                    | die Zahlen unten stehen gegen die Modellrechnung   |

### Der sechste Satz ist kein Rest

Fünf Schulen, sechs Instanzen. Der naheliegende Ausweg wäre, eine Schule zweimal zu
bedienen — das widerspricht aber „jeder Satz zielt auf eine Schule ... auch ungenutzte
Zauber bekommen ihre Nische": Zwei Sätze auf dieselbe Schule bedeuten, dass eine andere
leer ausgeht oder der zweite nur eine schwächere Kopie ist.

Der sechste zielt deshalb auf die einzige Größe, die **alle** Schulen teilen: den
globalen Cooldown. Er hilft jeder Zusammenstellung gleich und zwingt zu keiner.

> **Verworfen am 2026-08-11: ein Satz, der Schulenvielfalt belohnt.** Mein erster
> Entwurf gab dem sechsten Satz einen Effekt für „vier Zauber aus vier Schulen in
> Folge". Der Auftraggeber hat das abgelehnt, und die Begründung ist grundsätzlicher
> als der Effekt: **Kombinieren ist ohnehin frei** — jede Zusammenstellung, jede
> Talentverteilung steht offen. Es ist Aufgabe des Spielers, die Zauber zu finden, die
> zusammen tragen. Eine Mechanik, die für das Mischen belohnt, macht aus einer freien
> Entscheidung eine gelenkte und wäre eine Antwort auf eine Frage, die das Spiel
> bereits offenlässt.
>
> Zusätzlich hätte sie als einziger der sechs Effekte **neue Mechanik** gebraucht
> (einen Zähler über die letzten Würfe und ihre Schulen).

## Die Zahlen — und warum sie bewusst unter dem Affix-Aufbau liegen

|       Stufe | Bonus                             |     Kumuliert |
| ----------: | --------------------------------- | ------------: |
| **2 Teile** | +10 % Ausrüstungs-Anteil (Topf 4) |          10 % |
| **4 Teile** | +20 % Ausrüstungs-Anteil          |      **30 %** |
| **6 Teile** | Rotations-Effekt, **kein Anteil** | 30 % + Effekt |

Gegen die Modellrechnung des Plans bei Budget 0,30:

```
Affix-Aufbau   1,331      (drei Affixe, multiplikativ)
Satz-Aufbau    1,300      (30 % additiv)
Differenz      −2,3 %
```

**Das ist Absicht.** Der Satz liegt in reinen Werten leicht hinten, und der
Sechser-Effekt schließt die Lücke — genau die Rolle, die der Plan ihm gibt
(„der Sechser-Bonus als Krönung und Statussymbol"). Läge er schon bei den Werten
vorn, wäre die Abwägung entschieden, bevor sie beginnt, und der Plan verlangt
ausdrücklich, dass sie echt bleibt.

Die 10/20 decken sich mit dem, was Worker-2 in #748 bereits gemessen hat
(0 / 0 / 100 / 100 / 300 Promille bei null bis vier Teilen) — der Prüfsatz trägt
diese Kurve also schon, ohne dass sie je entschieden wurde.

## Die sechs Sätze

Namen englisch (Gesetz 5), alchemistisch im Setting des Werks.

|   # | Name                  | Instanz | Schule            | Stufe 6 — Rotations-Effekt                                                                                                                         |
| --: | --------------------- | ------: | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
|   1 | **Ashen Regimen**     |       9 | CALCINATION       | **PYROCLASM lässt ein Drittel der Hitze stehen.** Aus „alles oder nichts" wird eine Kette: entladen, nachlegen, wieder entladen                    |
|   2 | **The Long Grip**     |      10 | FIXATION          | **Der erste Kontroll-Ausgeber nach jedem Phasenwechsel kostet nichts.** Belohnt, den Moment zu treffen, statt Punkte zu horten                     |
|   3 | **Perpetual Draft**   |      11 | SUBLIMATION       | **Der Rücksetzer wirkt auf die zwei längsten Abklingzeiten statt auf eine.** Macht lange Zauber planbar statt zufällig                             |
|   4 | **The Sealed Vessel** |      12 | COAGULATION       | **Überschüssiger Schutz über der Obergrenze wird zu Schaden**, statt zu verfallen. Schließt den Kreis: Überleben zahlt auf Schaden ein             |
|   5 | **Slow Rot**          |      13 | PUTREFACTION      | **Zersetzungs-Stapel verfallen nicht, solange ein DoT läuft.** Der Aufbau vor dem Schadensfenster wird zur Kernentscheidung statt zum Wettlauf     |
|   6 | **The Whole Work**    |      14 | _schulunabhängig_ | **Der globale Cooldown sinkt um 100 ms.** Wirkt auf jede Zusammenstellung gleich, zwingt zu keiner — die letzte Instanz gibt Tempo, keine Richtung |

### Warum diese fünf Effekte und nicht Zahlen

Jeder greift in **die Entscheidung** ein, die der Plan als Kern der jeweiligen Schule
benennt, nicht in ihre Stärke:

- CALCINATION: „_wann_ entladen wird, ist echte Planung" → der Effekt macht aus einer
  Entscheidung eine Folge von Entscheidungen.
- FIXATION: „ein Planungswerkzeug statt einer passiven Wirkung" → der Effekt belohnt
  Timing, nicht Vorrat.
- COAGULATION: die Schule hat drei Wege; der Effekt verbindet den defensiven mit dem
  offensiven, ohne einen zu ersetzen.
- PUTREFACTION: „allein gespielt schwach — braucht immer Partner" → der Effekt nimmt
  den Zeitdruck, nicht die Abhängigkeit.
- SUBLIMATION: der Rücksetzer ist laut Plan ein eigener Zauber neben der laufenden
  Verkürzung; der Effekt schärft ihn, statt die Verkürzung zu erhöhen.

## Was ich NICHT entschieden habe

| Offen                                          | Warum                                                                                                                                                                                                                                   |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Welche Gegenstände zu welchem Satz gehören** | Heute tragen alle 21 Einträge `setId` = kein Satz. Die Zuordnung ist Katalog-Arbeit und hängt an den Endgame-Instanzen, die es noch nicht gibt                                                                                          |
| **Ob Stufe 6 überhaupt erreichbar ist**        | Sechs Satzteile brauchen sechs Plätze aus derselben Instanz — das setzt voraus, dass eine Endgame-Instanz mindestens sechs verschiedene Plätze abwirft                                                                                  |
| **Die genaue Wirkung in Zahlen**               | „ein Drittel der Hitze", „2 s" sind Startwerte für die Messung, keine Balance-Entscheidung. Der Plan verlangt den Prüflauf „bester Satz-Aufbau gegen besten Affix-Aufbau je Ausrichtung" — erst der sagt, ob die Werte stimmen          |
| **Wie stark 100 ms wirklich sind**             | Der GCD ist die Taktgröße des ganzen Spiels; 100 ms weniger heben JEDEN Zauber. Der Wert ist der vorsichtigste, der noch spürbar ist — er gehört als erster in den Prüflauf, weil er als einziger nicht auf eine Schule begrenzt bleibt |

## Der Nachweis, den keiner dieser Effekte heute hätte

`activeRotationEffects` ist gebaut und hat **keinen Aufrufer** (Worker-2, 2026-08-10).
Ein Katalog-Eintrag allein ändert das nicht. Wer diese sechs baut, braucht zuerst die
Leitung von der Satzstufe zur Wirkung — sonst steht am Ende ein Katalog, der
dokumentiert ist und nicht greift. Das ist derselbe Zustand, der an einem einzigen Tag
fünfmal gemeldet wurde.
