# Was beim Auftraggeber liegt

**Stand 2026-08-05, 17:05: Vier der ursprünglichen fünf Entscheidungen sind getroffen.** Sie stehen
unten mit dem Ergebnis und wandern ins GAME-DESIGN; was sie im Code auslösen, ist als Issue verlinkt.
**Vier Punkte sind seither dazugekommen** — alle vier aus der Arbeit selbst, nicht aus dem Interview.

|                                     | Ergebnis                                                                                                    |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| **1** Doppelbelegung                | **verboten** — ein Zauber liegt auf höchstens einem Platz                                                   |
| **2** Punktekurve                   | **offen, und anders zugeschnitten** — gehört gemessen, nicht entschieden                                    |
| **3** `DECAY_STACK_PERCENT`         | **auf 6 % anheben**                                                                                         |
| **4** Stumme Zauber                 | **der Auftraggeber bestätigt Suno einmal im sichtbaren Browser**                                            |
| **5** PUTREFACTION-Farbe            | **hell-violett bleibt** — der ganze Bildbestand ist so abgenommen                                           |
| **6** Wirkzeit gegen Tempo          | **OFFEN** — ab 33,4 % CATALYSIS ist ein Wirkzeit-Zauber nicht mehr abbrechbar                               |
| **7** Grundwerte je Fund            | **OFFEN, als Balance-Messung** — wie viel ESSENCE trägt ein Stück auf Stufe 1                               |
| **8** Widerstand gegen Schul-Stärke | **OFFEN** — der Korridor 20–30 % kann die Rotation nicht drehen, solange die Schulen 67 % auseinanderliegen |
| **9** WARD ohne Balance-Deckel      | **OFFEN** — eine hinreichend gute Ausrüstung kann den Spieler unverwundbar machen                           |

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

## 2. Wie verläuft die Punktekurve? — ENTSCHIEDEN: quadratisch (2,0)

**Entschieden am 2026-08-05**, an gerechneten Kurven statt an Vorschlägen — genau so, wie es der
Auftraggeber verlangt hatte.

| Exponent            | 5 s    | 20 s    | 50 s      | sehr gut (45 s) ÷ mittelmäßig (20 s) |
| ------------------- | ------ | ------- | --------- | ------------------------------------ |
| moderat 1,6         | 105    | 965     | 4.183     | 3,66×                                |
| **quadratisch 2,0** | **50** | **800** | **5.000** | **5,06×**                            |
| kubisch 3,0         | 6      | 400     | 6.250     | 11,39×                               |

**Die Begründung ist die Mitte:** Feinoptimierung lohnt deutlich, ein mittelmäßiger Lauf ist nicht
wertlos. Bei 1,6 lohnt die Feinoptimierung zu wenig — und genau dafür ist die Kurve da. Bei 3,0
spielt umsonst, wer die letzten Sekunden nicht holt.

**Dazu entschieden: der Rest-Ertrag bei 20 Promille** (GAME-DESIGN 12.6, „plus ein Rest-Ertrag fürs
bloße Wiederholen"). Fünfzig Wiederholungen ohne Verbesserung wiegen einen ersten Sieg auf —
spürbar weniger als Verbessern, aber nicht sinnlos.

Beide Werte stehen in `data/player/reward.json` und sind dort **nicht mehr als vorläufig**
gekennzeichnet.

---

_Der ursprüngliche Verlauf, zur Nachvollziehbarkeit:_

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

## 8. Der Widerstands-Korridor gegen die Schul-Stärke — OFFEN

**Blockiert:** nichts unmittelbar · **hält auf:** die Absicht hinter den Widerständen (#328, #268)

Gemessen von Worker-3 am 2026-08-05, nachdem die Widerstände auf beiden Seiten gebaut waren und
am Zauber hingen. Sieben Faktoren in einem Lauf:

```
Widerstand   Schaden   beste Rotation
1,00          31120    pyre / smoulder / kindle
0,75          23340    pyre / smoulder / kindle     ← Korridor endet hier
0,60          18668    pyre / smoulder / kindle
0,50          13480    smoulder / bind              ← ANDERS
0,20          12000    bind
```

**Die Mechanik wirkt. Der Korridor nicht.** Die Wahl kippt bei 0,50; das Design nennt 0,70–0,80.

### Warum

```
CALCINATION    kindle 100 · smoulder 40/Tick · pyre 12/Hitzepunkt
FIXATION       bind 60
PUTREFACTION   fester 60 · canker 45
```

Zwischen den schadensfähigen Schulen liegen **67 %**. Auch um ein Viertel gemindert steht KINDLE
vor BIND. Ein Widerstand von 25 % kann diesen Abstand nicht drehen — er müsste bei 40 % liegen.

> **Die zehn Zauber ohne Schaden sind kein Balance-Fehler.** Koagulation ist Überleben, Sublimation
> ist Cooldown-Manipulation. Sie kommen in einer Rotationssuche nicht vor, weil der Prüf-Boss ein
> reiner Lebensbalken ist — gegen einen Boss, der nichts tut, ist ein Schutzschild wertlos. Das ist
> ein Grund mehr für Bösse, die etwas verlangen, kein Grund, die Zauber zu ändern.

### Die zwei Sätze, die zusammen nicht aufgehen

|                      |                                                                                                                                                           |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **GAME-DESIGN 1048** | „Es muss möglich sein, nicht nötig." Wer bei seiner Lieblingsschule bleibt, kommt durch — nur langsamer. **Der Widerstand soll die Wahl nicht erzwingen** |
| **Leitplanke 2**     | „Jeder Zauber muss irgendwo die beste Wahl sein." **Der Widerstand soll die Wahl beeinflussen**                                                           |

Beide gelten, und beide sind richtig gemeint. Sie gehen nur zusammen, wenn die schadensfähigen
Schulen **ähnlich stark** sind: Liegen sie eng beieinander, verschiebt ein Widerstand von 25 % die
Rangfolge — liegen sie weit auseinander, verschiebt er gar nichts.

### Die Wahl

|                                      | Was es bedeutet                                                                                                                                                                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Die Schulen enger zusammenrücken** | Ein Zauber der einen Schule ist etwa so stark wie der entsprechende der anderen. Dann greift der Korridor — aber die Schulen unterscheiden sich nur noch in der **Mechanik**, nicht mehr in der Stärke. Das ist vermutlich ohnehin gewollt |
| **Den Korridor öffnen**              | 40–50 % statt 20–30 %. Dann dreht der Widerstand die Wahl auch bei ungleichen Schulen — aber „wer nicht umbaut, kommt durch, nur langsamer" wird zu „deutlich langsamer"                                                                   |
| **Beides so lassen**                 | Der Widerstand bleibt eine Zahl am Ende, keine Entscheidung davor. Dann fehlt der Mechanismus, der die Zauber-Vielfalt tragen soll — und Leitplanke 2 braucht einen anderen                                                                |

**Der Orchestrator neigt zur ersten Variante**, weil sie dem entspricht, was das Design ohnehin
über die Schulen sagt: Sie sind **mechanische Rollen**, nicht Stärkestufen. Wenn CALCINATION
grundsätzlich mehr Schaden macht als PUTREFACTION, ist die Schulwahl schon vor jedem Widerstand
entschieden.

**Das ist ein Verdacht, keine Entscheidung.** Die Frage wird scharf, sobald E10 die restlichen 45
Zauber entwirft — dann werden die Werte gesetzt, und bis dahin sollte klar sein, gegen welche
Messlatte.

## 9. WARD hat keinen Balance-Deckel — OFFEN

**Blockiert:** nichts heute · **hält auf:** E6, sobald Ausrüstung erspielt wird

Gefunden von Worker-1 bei #282, als er WARD an den Kampf anschloss — und ausdrücklich **nicht**
entschieden, sondern gemeldet. Das war richtig.

### Was gebaut ist

WARD mindert **additiv** zur Stufen-Basis; beides ist Minderung, also derselbe Topf (13.1b).

> Multiplikativ wäre nicht nur eine andere Zahl, sondern eine andere **Aussage**: Zwei Minderungen
> von je 50 % ergäben dann 75 % statt 100 %. Die Gegenprobe (auf multiplikativ umgestellt) färbt
> genau diesen Fall rot — ein reiner Größenvergleich hätte es nicht gezeigt, weil beide Fassungen
> „kleiner als ohne" sind.

### Was fehlt

Der einzige Deckel liegt bei **1,0**, und das ist eine **Rechenregel**, keine Balance-Grenze:
Darüber würde der Spieler von jedem Treffer geheilt.

**Ein Balance-Deckel darunter — etwa 80 % — steht nirgends im Design.** Solange er fehlt, kann eine
hinreichend gute Ausrüstung den Spieler **unverwundbar** machen.

### Die Ironie steht bereits im Quelltext

`combat.ts:792` begründet, warum die WARD-**Basis** nicht mit der Stufe skaliert:

> Ein Prozentsatz, der mitwächst, erreicht irgendwann 100 % und macht den Spieler unverwundbar.

**Genau diese Sorge trifft jetzt die Ausrüstung — und dort ist sie nicht abgefangen.** Die
Gefahr wurde einmal erkannt, an einer Stelle gelöst, und die zweite Stelle ist später dazugekommen.

### Was zu entscheiden ist

|                                 | Was es bedeutet                                                                                                                           |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **Ein harter Deckel**           | Wie bei CATALYSIS (50 %) und PURITY (100 %) — sichtbar in der Werte-Übersicht, mit Warnhinweis. Passt zu 13.6: „Die Deckel sind sichtbar" |
| **Abnehmende Erträge bei WARD** | Widerspricht 13.6 ausdrücklich („Keine abnehmenden Erträge. Jeder Prozentpunkt zählt gleich, bis zum Deckel")                             |
| **Kein Deckel**                 | Unverwundbarkeit ist erreichbar. Das ist eine Entscheidung, keine Lücke — aber es muss eine sein                                          |

**Der Orchestrator neigt zum harten Deckel**, weil das Design für die anderen zwei gedeckelten
Werte bereits genau dieses Muster festlegt und die Sichtbarkeit mitbeschreibt. Die **Höhe** ist
dann wieder eine Balance-Zahl und gehört gemessen — wie Punkt 7.

**Heute folgenlos:** Es gibt keine erspielbare Ausrüstung. Die Frage wird scharf, sobald E6 die
Fundstücke baut.

---

## 11. Zwei Zauber der Bauart „Abklingzeit verkürzen" sind beide tot — OFFEN

Gemessen am 2026-08-05 von Worker-1 mit dem Balance-Läufer, an zwei Bossen mit
verschiedener Dauer und verschiedener bester Rotation:

```
first-crucible (60 s)    nie gewirkt:  draught,  vapour, (+12 weitere)
test-metronome (300 s)   nie gewirkt:  draught,  vapour, (+12 weitere)
```

`draught` war ein Entwurf für #268: ein GCD (100 Schaden) gegen **5000 ms auf alle
Abklingzeiten**. `VAPOUR` ist der ausgelieferte Sublimations-Zauber mit derselben
Rechnung. **Beide kommen in keiner gemessenen Rotation vor.**

> Der Befund ist nicht „`draught` ist schlecht entworfen". Er lautet: **Die Bauart
> ‚Abklingzeit verkürzen' trägt in diesem Kampf nicht** — es laufen zu wenige lange
> Abklingzeiten gleichzeitig, als dass ihre Verkürzung einen GCD zurückverdiente.

### Warum das eine Design-Frage ist und keine Balance-Frage

`SUBLIMATION` ist eine der fünf Schulen, und ihre **zentrale Mechanik ist genau diese
Bauart** (GAME-DESIGN: „Automatische Verkürzung aller Abklingzeiten"). Trägt sie nicht,
trägt die Schule nicht — und dann sind es nicht zwei tote Zauber, sondern zwölf.

### Was die Messung NICHT sagt

Sie lief an **zwei** Bossen mit **15** Zaubern. Das fertige Spiel hat 60 Zauber und
Abklingzeiten bis 180 Sekunden. Die Aussage „es laufen zu wenige lange Abklingzeiten
gleichzeitig" ist eine Aussage über **heute**, nicht über das Spiel.

**Genau deshalb liegt der Punkt hier und ist nicht entschieden:** Er könnte sich von
selbst erledigen, wenn die Zauberzahl wächst — oder er könnte bedeuten, dass eine
Schule auf einer Mechanik steht, die nie trägt. Das ist zu wichtig, um es dem Zufall zu
überlassen, und zu früh, um es jetzt zu beantworten.

### Die Wahl

|                                               | Was es bedeutet                                                                                                                                                         |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Abwarten und bei 60 Zaubern erneut messen** | Kostet nichts. Risiko: Die Schule ist dann gebaut, und ein Befund an der Wurzel trifft zwölf Zauber und drei Talentzweige                                               |
| **Die Mechanik jetzt schärfen**               | Etwa: Verkürzung **prozentual** statt in Millisekunden, oder gezielt auf den **nächsten** Zauber statt auf alle. Dann hängt ihr Wert nicht daran, wie viel gerade läuft |
| **Die Schul-Mechanik austauschen**            | Der härteste Schnitt. `SUBLIMATION` behielte ihren Namen und bekäme eine andere zentrale Mechanik                                                                       |

**Bis zur Entscheidung:** `draught` ist aus #268 gestrichen — nicht wegen dieses Punktes,
sondern weil er Sublimations-Mechanik in einem Kalzinations-Zauber war. Der Prüffall
`genau_ein_ausgelieferter_zauber_verkuerzt_etwas` hat das gefangen.
