# Was beim Auftraggeber liegt

**Stand 2026-08-05, 17:05: Vier der ursprünglichen fünf Entscheidungen sind getroffen.** Sie stehen
unten mit dem Ergebnis und wandern ins GAME-DESIGN; was sie im Code auslösen, ist als Issue verlinkt.
**Zwei Punkte sind seither dazugekommen** — beide aus der Arbeit selbst, nicht aus dem Interview.

|                             | Ergebnis                                                                      |
| --------------------------- | ----------------------------------------------------------------------------- |
| **1** Doppelbelegung        | **verboten** — ein Zauber liegt auf höchstens einem Platz                     |
| **2** Punktekurve           | **offen, und anders zugeschnitten** — gehört gemessen, nicht entschieden      |
| **3** `DECAY_STACK_PERCENT` | **auf 6 % anheben**                                                           |
| **4** Stumme Zauber         | **der Auftraggeber bestätigt Suno einmal im sichtbaren Browser**              |
| **5** PUTREFACTION-Farbe    | **hell-violett bleibt** — der ganze Bildbestand ist so abgenommen             |
| **6** Wirkzeit gegen Tempo  | **OFFEN** — ab 33,4 % CATALYSIS ist ein Wirkzeit-Zauber nicht mehr abbrechbar |
| **7** Grundwerte je Fund    | **OFFEN, als Balance-Messung** — wie viel ESSENCE trägt ein Stück auf Stufe 1 |

---

## 1. Darf derselbe Zauber auf zwei Plätzen liegen? — ENTSCHIEDEN: nein

**Ergebnis vom 2026-08-05: Doppelbelegung ist verboten.** Ein Platz ist knapp (12 bis 16 bei 60
Zaubern), und ihn zu verdoppeln verschenkt ihn.

**Was daraus folgt:**

|                     |                                                                                                                                                                                                        |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **#287**            | AC3 ist der zu bauende Zweig: Die Auswahl bietet einen bereits gelegten Zauber nicht erneut an, und ein Spielstand mit Doppelbelegung wird beim Laden bereinigt. AC2/AC4 sind bereits gebaut (PR #288) |
| **#306**            | Die Auflage zu AC2 entfällt — die Reparatur an `bind()` muss nicht mehr unter zwei Antworten richtig sein. Das war der Grund, aus dem eine Sitzung das Issue abgelehnt hat                             |
| **GAME-DESIGN 715** | trägt die Regel                                                                                                                                                                                        |

Die Zählung „wie viele Zauber zahlen je Mechanik ein" (Kapitel 16) stimmt damit von selbst.

## 2. Wie verläuft die Punktekurve? — OFFEN, und anders zugeschnitten

**Blockiert:** #276 · **hält auf:** E5 insgesamt

Es gibt keine Punkte. `score` ist ein Strich:

```
$ grep -rn "score" src/render/overlay.ts src/analysis/save-format.ts
  save-format.ts   /score(?!board)/i,       ← verbotenes Feld im Spielstand
  overlay.ts       ['score', 'Score'],      ← die Beschriftung
  overlay.ts       set('score', '—');       ← der Wert
```

Die Zeile `Score` ist die Punkte-Anzeige aus der Kurzfassung nach dem Kampf — belegt, nicht
geschlossen:

```
GAME-DESIGN 1349   Dann die Kurzfassung: Erfolg/Misserfolg, Restzeit, PUNKTE, drei Kennzahlen
Plan, Zeile 1093   dieselbe Aufzählung
Plan, Zeile 53     die Restlaufzeit beim Boss-Sieg wird in Punkte umgerechnet
```

### Die Frage war falsch gestellt

Der Orchestrator hat drei Kurven zur Wahl gestellt — quadratisch, kubisch, exponentiell. Antwort des
Auftraggebers am 2026-08-05:

> Das muss eigentlich durch unsere Simulation getestet werden, damit die Balance passt. Weil wir ja
> nicht wissen, ob die von dir vorgeschlagenen Settings nicht zu viele oder zu wenige Punkte dann
> geben.

**Das trifft.** Die Kurve entscheidet nicht über den Charakter des Spiels, sondern über eine Zahl —
und ob sie zu viel oder zu wenig ausschüttet, kann man nicht wissen, sondern nur messen.

### Was heute messbar ist und was nicht

|                   |                                                                                                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **nicht messbar** | Ob die Ausschüttung „passt". Dafür fehlt die andere Seite: was Punkte **kosten** (Instanz-Freischaltungen, Aufwertungsstufen). Beides gibt es noch nicht                 |
| **messbar**       | Das **Verhältnis**: Wie viel mehr bekommt ein sehr guter Lauf gegenüber einem mittelmäßigen? Das beantwortet, ob Feinoptimierung sich lohnt — und dafür ist die Kurve da |

**Die Entscheidung bleibt damit offen, aber sie wartet nicht mehr auf dich, sondern auf eine
Messung.** Als Auflage an #276 nachgetragen.

## 3. Bleibt `DECAY_STACK_PERCENT` bei 3? — ENTSCHIEDEN: auf 6 %

**Ergebnis vom 2026-08-05: 6 % je Stapel.** Bei drei erreichbaren Stapeln sind das 18 % statt 9 % —
über dem Break-even von 5,4 %, ab dem sich der Leisten-Platz rechnet.

**Der Grund:** Bei 3 % wäre PUTREFACTION **nie** die beste Wahl, und das ist etwas anderes als
„allein gespielt schwach". Dein Gesetz sagt: jeder Zauber muss irgendwo die beste Wahl sein.

**Was daraus folgt:** Der Wert steht auf **beiden** Seiten (`src/core/combat.ts`,
`sim/src/fight.rs`) und muss dort gleichzeitig geändert werden; danach werden die Vergleichsdateien
neu exportiert. Die Begründung steht in GAME-DESIGN, Kapitel 7.

## 4. Wie kommen die sieben fehlenden Klänge zustande?

**Blockiert:** #198 und #204 vollständig

Sieben Zauber sind stumm, und das Manifest hält sie für fertig:

| Eintrag                                                   | Datei | als vorläufig markiert |
| --------------------------------------------------------- | ----- | ---------------------- |
| `spell-crust`, `spell-salve`, `spell-temper`              | fehlt | **nein**               |
| `spell-bind`, `spell-stall`, `spell-stay`, `spell-anchor` | fehlt | **nein**               |

Am Guthaben liegt es nicht:

```
$ /venv/bin/python tools/suno_api.py auth-test
Account: backupslave  |  billing/info: HTTP 200
  Credits: 2322  Tier: True
```

Es liegt an der Bot-Erkennung vor dem Erzeugen. **Der Weg, sie zu umgehen, ist abgelehnt und bleibt
es** — hier steht nur, welche Wege offen sind:

| Weg                                | Was der Auftraggeber tut                                                                       | Preis                                                                                 |
| ---------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **einmal bestätigen**              | Suno im sichtbaren Browser öffnen, die Prüfung anklicken, den Lauf dahinter durchlaufen lassen | ein paar Minuten Anwesenheit, einmal je Sitzung                                       |
| **Freesound statt Suno**           | nichts                                                                                         | anderer Klangcharakter als die 56 vorhandenen; CC0/CC-BY prüfen                       |
| **sieben als vorläufig markieren** | nichts                                                                                         | die Zauber bleiben stumm, aber der Wächter meldet es nicht mehr fälschlich als fertig |

Der dritte Weg ist **kein Ersatz**, sondern ehrliche Buchführung: Er behebt, dass sieben unfertige
Einträge als fertig gelten, und lässt den Mangel selbst bestehen.

---

## 5. Bleibt PUTREFACTION hell-violett? — ENTSCHIEDEN: ja

**Ergebnis vom 2026-08-05:** Der gebaute Wert `#9b6fd0` bleibt, und GAME-DESIGN wird darauf
nachgezogen. Die Entscheidung fiel im Rahmen einer Durchsicht **aller 68 Bilddateien** des Projekts
(`localhost:8000/rota-bilder/`) — Wortlaut: „die bilder passen so fürs erste. wir können ja später
noch immer neue mit ki generieren."

**Was daraus folgt:**

- Kein Code ändert sich; alle fünf Schulen bleiben im selben Kontrast-Korridor.
- **Der Bildbestand ist damit abgenommen, aber nicht angeschlossen:** 24 Zauber-Symbole und zwei
  erzeugte Rahmen liegen im Repository und werden nirgends geladen. Eigener Befund, eigenes Issue.
- KI-Erzeugung weiterer Bilder ist ausdrücklich für **später** vorgemerkt, nicht für jetzt.

## Was hier bewusst **nicht** steht

- **Die Gestaltung der sechs neuen Anzeigen aus #294** — das ist Umsetzung und gehört dem Bearbeiter.
- **KINDLEs Abklingzeit** — `cooldownMs: 1500` ist der GCD und damit keine offene Frage, sondern die
  Messlatte für jeden weiteren Kalzinations-Zauber (#268).
- **Alles aus GAME-DESIGN 21** — dort steht, was bereits verworfen wurde. Nichts davon wird ohne
  ausdrückliche Rücksprache wieder aufgemacht.

---

## 6. Wirkzeit gegen Tempo — OFFEN, und niemand hat es entschieden

**Blockiert:** nichts heute · **hält auf:** E10, sobald Zauber Wirkzeiten bekommen (#268)

Gemeldet von Worker-3 beim Bau von #304, vom Orchestrator nachgerechnet. Zwei Regeln des
GAME-DESIGN, einzeln beide richtig:

|         |                                                                                              |
| ------- | -------------------------------------------------------------------------------------------- |
| **6.6** | die **Wirkzeit** skaliert mit Tempo                                                          |
| **6.2** | die **globale Wartezeit** skaliert **nicht** mit Ausrüstungs-Tempo — nur über den Talentbaum |

Ein laufender Zauber wird von jeder anderen Wirkung abgebrochen. Aber ein Druck, den die Wartezeit
abweist, ist keine Wirkung — das Fenster für den Abbruch ist also die **Differenz** der beiden
Zeiten. Schrumpft nur eine Seite, schließt es sich:

```
CATALYSIS   Wirkzeit (2000 ms Basis)   Fenster   mit GCD-Talent (1000 ms)
   0 %              2000                500 ms         1000 ms
  20 %              1670                170 ms          670 ms
  33 %              1510                 10 ms          510 ms
  40 %              1430                  0             430 ms
  50 %              1340                  0             340 ms   ← Deckel
```

**Ab 33,4 % CATALYSIS ist ein Wirkzeit-Zauber von keinem anderen Zauber mehr abbrechbar** — nur
noch vom Unterbrecher, der als einziger keine Wartezeit auslöst. Das liegt weit unter dem Deckel
von 50 %, ist im Endgame also der Normalfall.

**Ein längerer Zauber löst es nicht:** Selbst am oberen Ende des Korridors (2,5 s) bleiben bei
vollem Tempo 170 ms.

### Die Wahl

|                                       | Was es bedeutet                                                                                                                                                                    |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Das GCD-Talent trägt die Mechanik** | Wer Wirkzeit-Zauber spielen will, muss in den GCD investieren. Eine echte Kopplung zwischen Talentbaum und Zauberwahl — reizvoll, aber sie schränkt die freie Zusammenstellung ein |
| **Die Folge annehmen**                | Tempo-Ausrichtungen verlieren die Abbruch-Entscheidung. Wirkzeit-Zauber werden für sie zu reinen „langsamen Zaubern" ohne die Verpflichtung aus 7.1                                |

**Heute folgenlos:** Der Prüf-Zauber aus #304 ist die einzige Wirkzeit im Spiel und für keinen
Spieler erreichbar. Die Frage wird scharf, sobald E10 Wirkzeiten vergibt.

## 7. Wie viel trägt ein Fund? — OFFEN, aber als Messung, nicht als Entscheidung

**Blockiert:** #282 teilweise · **hält auf:** E6 insgesamt

Bei #282 gemessen: `BASE_STAT_IDS` steht auf beiden Seiten und **wird nirgends gelesen**.

```
WornValues (Rust und TS)   catalysis · purity · yield_bonus · gear_effects
                           KEIN essence, vessel, ward
data/gear/                 existiert nicht — es gibt keine Gegenstands-Daten
```

Ein Gegenstand kann ESSENCE tragen; im Kampf kommt davon nichts an. **Jeder Fund verbessert also
spürbar nichts** — und das Gegenteil ist im Design der Grund, warum Grinden befriedigend sein soll.

### Was bereits entschieden ist und nicht neu aufgemacht wird

- **Nicht am Platz** — die Plätze sind Behälter (GAME-DESIGN 13.2, dort 2026-07-29 gestrichen)
- **An der Gegenstandsstufe**, mit der Kurve, die es schon gibt (`LEVELS.factorPerMille`)
- **Die Qualität steuert die Affix-Zahl**, nicht die Grundwerte — das ist eine **Auslegung** des
  Orchestrators und im GAME-DESIGN als solche gekennzeichnet

### Was offen ist

**Die Basis: wie viel ESSENCE ein Gegenstand auf Stufe 1 trägt.** Eine reine Balance-Zahl.

Sie geht denselben Weg wie Punkt 2: **nicht an den Auftraggeber zum Raten, sondern an den
Balance-Läufer zum Messen.** Er weist Skalierungsfaktoren je Wert aus — die Frage lautet, bei
welcher Basis ein Ausrüstungsstück spürbar etwas beiträgt, ohne die Sonderwerte zu erdrücken.

Bis dahin steht die Zahl als benannte, ausdrücklich unkalibrierte Konstante an einer Stelle, und
der Prüffall misst das **Verhältnis** (doppelte Stufe, doppelter Wert) statt des Betrags — sonst
meldete er rot, sobald jemand kalibriert, und arbeitete gegen die Balance-Arbeit statt für sie.
