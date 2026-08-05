# Was beim Auftraggeber liegt

**Stand 2026-08-05, 15:10: Vier der fünf Entscheidungen sind getroffen.** Sie stehen unten mit dem
Ergebnis und wandern ins GAME-DESIGN; was sie im Code auslösen, ist als Issue verlinkt. Offen ist
nur noch eine — und sie hat sich dabei von einer Design- in eine Balance-Frage verwandelt.

|                             | Ergebnis                                                                 |
| --------------------------- | ------------------------------------------------------------------------ |
| **1** Doppelbelegung        | **verboten** — ein Zauber liegt auf höchstens einem Platz                |
| **2** Punktekurve           | **offen, und anders zugeschnitten** — gehört gemessen, nicht entschieden |
| **3** `DECAY_STACK_PERCENT` | **auf 6 % anheben**                                                      |
| **4** Stumme Zauber         | **der Auftraggeber bestätigt Suno einmal im sichtbaren Browser**         |
| **5** PUTREFACTION-Farbe    | **hell-violett bleibt** — der ganze Bildbestand ist so abgenommen        |

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
