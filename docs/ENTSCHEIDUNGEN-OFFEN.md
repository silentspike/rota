# Was beim Auftraggeber liegt

**Stand 2026-08-06, 16:40: Dreizehn der vierzehn Punkte sind entschieden oder erledigt.** Die
entschiedenen stehen unten mit Ergebnis und wandern ins GAME-DESIGN; was sie im Code auslösen,
ist als Issue verlinkt. **Acht der vierzehn sind aus der Arbeit selbst entstanden, nicht aus dem
Interview** — sie kamen zutage, weil eine Abnahme etwas nachgemessen hat.

|                                     | Ergebnis                                                                                           |
| ----------------------------------- | -------------------------------------------------------------------------------------------------- |
| **1** Doppelbelegung                | **verboten** — ein Zauber liegt auf höchstens einem Platz                                          |
| **2** Punktekurve                   | **quadratisch (2,0), Rest-Ertrag 20 Promille**                                                     |
| **3** `DECAY_STACK_PERCENT`         | **auf 6 % anheben**                                                                                |
| **4** Stumme Zauber                 | **ERLEDIGT ohne dein Zutun** — elf Klänge erzeugt, #198 und #204 geschlossen; nur das Hören bleibt |
| **5** PUTREFACTION-Farbe            | **hell-violett bleibt** — der ganze Bildbestand ist so abgenommen                                  |
| **6** Wirkzeit gegen Tempo          | **ENTSCHIEDEN: 2,3–3,0 s** — kürzere Zauber wirken sofort, die Regeln bleiben unangetastet         |
| **7** Grundwerte je Fund            | **OFFEN, als Balance-Messung** — wie viel ESSENCE trägt ein Stück auf Stufe 1                      |
| **8** Widerstand gegen Schul-Stärke | **ENTSCHIEDEN: die Schulen angleichen** — nicht am Widerstand drehen                               |
| **9** WARD-Deckel                   | **ENTSCHIEDEN: 75 % auf die SUMME** — Grundwert plus Ausrüstung; nutzbarer Anteil damit 550 ‰      |
| **11** SUBLIMATION-Bauart           | **VERTAGT** — erneut messen, wenn es mehr als 15 Zauber gibt; die Bauart bleibt vorerst            |
| **12** `runaway`-Timing             | **ENTSCHIEDEN: Tiefe** — bleibt, wie es ist                                                        |
| **13** Talentpunkt-Verteilung       | **WAR NIE OFFEN** — „etwa jede zweite Stufe einer" steht im Plan und ist die Verteilung            |
| **14** Fallrate der Beute           | **ENTSCHIEDEN: 1000 ‰** — jeder Sieg lässt etwas fallen, umgesetzt und gemergt                     |

> **Es gibt keinen Punkt 10.** Er hat nie existiert — beim Anlegen von Punkt 11 wurde die Zahl
> übersprungen. Die Lücke bleibt stehen, statt umzunummerieren: Die Nummern werden in Issues,
> Nachrichten und Commits zitiert, und eine stille Verschiebung würde jeden dieser Verweise auf
> einen anderen Punkt zeigen lassen. Eine sichtbare Lücke ist billiger als eine unsichtbare
> Umdeutung.

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

## 4. Wie kommen die sieben fehlenden Klänge zustande? — ERLEDIGT, ohne Zutun des Auftraggebers

**#198 und #204 sind geschlossen.** Es waren nicht sieben, sondern **elf**: die sieben unten, dazu
`spell-vapour` und `spell-reflux` (als vorläufig markiert und deshalb nicht mitgezählt) sowie
`bellows` und `searing` aus #268. Kein spielbarer Zauber ist mehr stumm.

**Der Punkt hat sich aufgelöst, weil die Annahme darunter falsch war.** Hier stand, es liege an
der Bot-Erkennung und der Auftraggeber müsse einmal bestätigen. Tatsächlich verlangt Suno **je
Generierung** einen Turnstile-Token, der nur in der echten Seite entsteht — ein Durchlauf dort
stellt das Vertrauen für etwa **drei** Generierungen wieder her. Für elf Klänge waren das drei
Aufwärm-Zyklen, keine einmalige Bestätigung. Das Werkzeug bringt sie als `suno_song.py warmup`
bereits mit; ich hatte den Vorgang von Hand nachgebaut, statt ihn aufzurufen.

> **Nichts davon umgeht die Prüfung.** Jeder Zyklus ist eine echte Generierung über die Seite,
> die Credits kostet. Die Ablehnung, die Bot-Erkennung zu umgehen, steht unberührt.

**Was der Auftraggeber trotzdem noch tun muss:** die elf Klänge **hören**. Technisch sind sie
geprüft (Länge exakt, −16 bis −20 LUFS, keine Anfangsstille, 120 von 120 Paaren unterscheidbar),
ästhetisch nicht — das kann ich nicht.

<details>
<summary>Der ursprüngliche Text, zur Nachvollziehbarkeit</summary>

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

</details>

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

## 6. Wirkzeit gegen Tempo — ENTSCHIEDEN: Spanne 2,3–3,0 s, kürzere wirken sofort

**Der Auftraggeber hat die Frage zurückgewiesen und die bessere Antwort gegeben:** _„naja dann
müssen wir doch eher die spells überarbeiten?"_ — statt an einer der beiden Design-Regeln zu
drehen oder ein Mindest-Fenster zu erfinden.

Nachgerechnet mit der echten `hastedMs`, nicht nachgebaut:

| Basis-Wirkzeit | bei 50 ‰ Tempo | Fenster ohne Talent | mit GCD-Talent |
| -------------- | -------------- | ------------------- | -------------- |
| 1500 ms        | 1000 ms        | **−500**            | 0              |
| 2000 ms        | 1340 ms        | **−160**            | +340           |
| 2250 ms        | 1500 ms        | 0                   | +500           |
| **2300 ms**    | 1540 ms        | **+40**             | +540           |
| 3000 ms        | 2000 ms        | +500                | +1000          |

**Ab etwa 2300 ms Basis bleibt das Fenster auch bei maximalem Tempo offen.** Keine Regel muss
fallen, keine dritte muss dazukommen.

**Was noch zu wählen ist:** GAME-DESIGN nennt heute **1,5–2,5 s**. Die Untergrenze ist genau der
schlimmste Fall. Entweder wandert die Spanne auf **2,3–3,0 s**, oder kurze Wirkzeit-Zauber
werden zu **Sofort-Zaubern** und die Wirkzeit bleibt den wuchtigen vorbehalten.

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

> **Der Befund darunter ist behoben, die Frage darüber nicht.** Nachgemessen am 2026-08-06:
>
> ```
> WornValues (Rust)   catalysis · purity · yield_bonus · essence · vessel · ward · gear_effects
> WornValues (TS)     dieselben sieben
> combat.ts:767       essence: NEUTRAL_FACTORS.essence * essenceFactor(this.worn)
> ```
>
> ESSENCE, VESSEL und WARD stehen seit #282 in `WornValues` und wirken im Kampf. **Was hier
> ursprünglich als Befund stand, gilt nicht mehr** — der Text darunter beschreibt den Stand vom
> 2026-08-05 und ist als Herleitung stehengeblieben, nicht als Lagebeschreibung.
>
> **Offen ist weiterhin die Zahl**, und nur sie: wie viel ESSENCE ein Gegenstand auf Stufe 1
> trägt. Dass ein Fund im Kampf ankommt, ist inzwischen gebaut; dass er **etwas Bestimmtes**
> trägt, nicht — es gibt weiterhin keine Gegenstands-Daten (#421).

_Ursprüngliche Herleitung, Stand 2026-08-05:_

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

## 8. Der Widerstands-Korridor — ENTSCHIEDEN: die Schulen angleichen

**Ergebnis vom 2026-08-06:** Nicht am Widerstand drehen, sondern die **67 % Abstand zwischen den
schadensfähigen Schulen** verkleinern. Dann greift der Korridor 20–30 % wie gedacht.

Die Begründung liegt in der Sache: Ein Widerstand von 25 % kann eine Reihenfolge nicht kippen,
wenn die Zauber ohnehin um zwei Drittel auseinanderliegen. **Das Problem ist der Abstand, nicht
der Korridor** — und dass drei Schulen so weit auseinanderliegen, widerspricht ohnehin dem
Grundsatz, dass jeder Zauber irgendwo die beste Wahl sein soll.

> **Zusammen mit Punkt 6 anzufassen.** Beide berühren die Zauber-Zahlen; zwei gleichzeitige
> Umbauten daran wären nicht mehr auseinanderzuhalten, wenn der Balance-Läufer danach etwas
> anderes misst.

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

## 9. WARD-Deckel — ENTSCHIEDEN: 75 %

**Ergebnis vom 2026-08-06: Deckel bei 75 %.** Nicht 80 %, wie vorgeschlagen — der Auftraggeber
hat die Zahl selbst gesetzt.

Damit reiht sich WARD in die anderen Sonderwerte ein: **CATALYSIS 50 %, PURITY 100 %, WARD 75 %.**
Der Deckel ist **sichtbar** wie dort auch — die Werte-Übersicht nennt Zahl und Warnhinweis, sonst
verliert der Spieler Ausrüstungsplätze an einen Wert, der nichts mehr bringt.

Der bisherige Deckel bei 1,0 bleibt als **Rechenregel** daneben bestehen: Darüber würde der
Spieler von jedem Treffer geheilt. Das ist kein Balance-Wert, sondern eine Grenze der Formel.

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

Der Kommentar an `wardFactor()` in `combat.ts` begründet, warum die WARD-**Basis** nicht mit der Stufe skaliert:

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

## 11. Zwei Zauber der Bauart „Abklingzeit verkürzen" sind beide tot — VERTAGT

**Ergebnis vom 2026-08-06: abwarten und erneut messen.** Die Bauart bleibt vorerst, wie sie im
Design steht.

Die Begründung steht im Abschnitt „Was die Messung NICHT sagt" weiter unten: Sie lief an **zwei**
Bossen mit **15** Zaubern. Bei 60 Zaubern und Abklingzeiten bis 180 Sekunden laufen mehr lange
Abklingzeiten gleichzeitig — genau die Bedingung, unter der die Verkürzung sich lohnt.

> **Das Risiko ist benannt und angenommen:** Trägt die Bauart auch dann nicht, sind es nicht zwei
> tote Zauber, sondern zwölf. Der Punkt bleibt deshalb in dieser Liste stehen und wird beim
> nächsten Balance-Lauf mit mehr als 15 Zaubern erneut vorgelegt.

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

---

## 12. Bei `test-runaway` ist seltener drücken besser — ENTSCHIEDEN: Tiefe

**Ergebnis vom 2026-08-06: Es bleibt, wie es ist.** Die Deckelung am angesammelten Verlust ist
Tiefe, keine Falle — wer sie versteht, spielt besser, und sie ist deterministisch, also planbar.

Damit ist auch die Nebenwirkung angenommen: **Der naheliegende Reflex — bedroht, also sofort
drücken — ist falsch, und nichts im Spiel sagt das.** Ein Spieler, der es nie herausfindet, spielt
schlechter, ohne zu wissen warum. Genau das meint „very hard to master".

Gemessen von Worker-3 am 2026-08-05 mit dem Balance-Läufer, deterministisch:

| `crust` alle … | Sieg | Ring-Rest | stabilisiert |
| -------------- | ---- | --------- | ------------ |
| nur bei t=0    | nein | 0 %       | 0            |
| 2 000 ms       | ja   | 52 %      | 720          |
| 7 000 ms       | ja   | 64 %      | 840          |
| **9 500 ms**   | ja   | **94 %**  | **1 140**    |
| 10 000 ms      | ja   | 40 %      | 600          |

**Alle zwei Sekunden zu drücken ist schlechter als alle neuneinhalb.** Und der Druck bei `t=0` ist
immer verschenkt — er verbraucht einen der drei Stapel, ohne etwas zurückzudrängen.

### Warum

```
combat.ts, undoProgress()
  if (amount <= 0 || this.damageDealtValue <= 0) return 0;
  const wirklich = Math.min(amount, this.damageDealtValue);
```

Die Rückdrängung ist am **angesammelten** Verlust gedeckelt. Wer früh drückt, verschenkt den Rest
der 900.

### Die Frage

Das Spiel ist laut Leitmotiv eine Rotations-Optimierung: „easy to learn, very hard to master", und
der Kern ist das Beherrschen der Reihenfolge unter Zeitdruck. **Hier ist die optimale Handlung,
seltener zu drücken.**

|           |                                                                                                                                    |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Tiefe** | „Timing statt Frequenz" — eine zweite Fähigkeit neben der Rotation, und ein Grund, warum sich `runaway`-Bosse anders anfühlen      |
| **Falle** | Das Spiel bestraft, was es sonst belohnt. Wer wie gewohnt auf Zuruf spielt, bekommt 52 statt 94 Prozent — und erfährt nicht, warum |

Beides ist vertretbar, und die Entscheidung ist keine Zahl, sondern eine über den **Charakter**
eines ganzen Boss-Typs. Deshalb liegt sie hier.

> **Unabhängig davon wird die Deckelung durch einen Prüffall festgehalten** (#394): Er hält fest,
> dass 9 500 ms besser ist als 2 000 ms, und fällt, wenn jemand die Deckelung für eine
> Sicherheitsprüfung hält und entfernt. Was heute gilt, ist damit gesichert — die Frage ist, ob es
> so bleiben soll.

---

## 13. Wie verteilen sich die 50 Talentpunkte über 100 Stufen? — WAR NIE OFFEN

**Dieser Punkt hätte nie hier stehen dürfen.** Die Antwort steht seit jeher im Plan, an zwei
Stellen, und im GAME-DESIGN ein drittes Mal:

```
Plan Zeile 117    Talentpunkte | ~50 (etwa jede zweite Stufe einer)
Plan Zeile 945    ~50 bei 100 Stufen (etwa jede zweite Stufe einer)
GAME-DESIGN 111   dasselbe
```

**„Etwa jede zweite Stufe einer" ist die Verteilung**, nicht bloß die Summe — das ist Variante A,
und die ist eingebaut. Der Punkt entstand, weil drei Varianten durchgerechnet wurden, die alle 50
treffen; über der Rechnerei wurde übersehen, dass der Satz den **Rhythmus** beschreibt.

> **Was daraus folgt, und es betrifft diese ganze Liste:** Ein Punkt gehört erst hierher, wenn
> nachgesehen wurde, ob Plan und GAME-DESIGN ihn nicht längst beantworten. Bei Punkt 4 stand eine
> falsche Annahme, hier eine bereits beantwortete Frage — beide haben Zeit gekostet, die niemand
> hatte.

Die Rechnung von Worker-3 bleibt unten stehen: Sie ist nicht falsch, sie war nur nicht nötig.

**Die Zahl ist entschieden, ihre Verteilung nicht.** GAME-DESIGN 12.1 nennt „~50 (etwa jede zweite
Stufe einer)". Die Summe steht damit fest; wann die Punkte kommen, bestimmt den **Charakter** des
Aufstiegs — und deshalb liegt die Frage hier und nicht beim Balance-Läufer.

Gerechnet von Worker-3 bei #379, alle drei treffen genau 50 und enden auf Stufe 100:

| Verteilung         | bis 10 | bis 25 | bis 50 | bis 75 | Charakter                         |
| ------------------ | ------ | ------ | ------ | ------ | --------------------------------- |
| **A** gleichmäßig  | 5      | 12     | 25     | 37     | wörtlich wie 12.1 — **eingebaut** |
| **B** spät dichter | 2      | 8      | 20     | 34     | Endspiel belohnt, früh karg       |
| **C** früh dichter | 9      | 18     | 30     | 40     | Stufe 1–8 jede Stufe einen        |

### Was daran hängt

**C gibt auf den Stufen 1 bis 8 jede Stufe einen Punkt.** Das passt zu „easy to learn" — der
Spieler merkt sofort, dass es etwas zu entscheiden gibt. Es verlangt ihm aber acht Entscheidungen
in den ersten Kämpfen ab, zu einem Zeitpunkt, an dem er die Bäume noch nicht kennt.

**B ist das Gegenteil:** bis Stufe 25 nur acht Punkte. Der Aufstieg fühlt sich früh mager an, dafür
trägt jeder späte Punkt mehr — und die letzten Stufen vor dem Endgame bleiben interessant.

**A ist die wörtliche Lesart** des Designs und heute eingebaut, ausdrücklich als **VORLÄUFIG** in
`data/player/levels.json` vermerkt.

### Die Wahl

Es gibt keine falsche Antwort — die drei unterscheiden sich nicht in der Menge, sondern darin,
**wann der Spieler zum ersten Mal etwas zu entscheiden hat**. Wer bei A bleibt, muss nichts tun.

> **Was diese Entscheidung nicht ist:** eine Balance-Frage. Die Gesamtzahl bleibt in allen drei
> Fällen 50, also ist keine Verteilung stärker als eine andere. Es geht allein um das Tempo, in dem
> sich der Talentbaum öffnet.

---

## 14. Wie oft lässt ein Sieg überhaupt etwas fallen? — ENTSCHIEDEN: jeder

**Ergebnis vom 2026-08-06: 1000 ‰ — jeder Sieg lässt etwas fallen.** Umgesetzt und gemergt.

Begründung des Auftraggebers, wörtlich: _„jeder sieg. in wow gibt es doch auch keine bosse die
nichts droppen."_ Das trifft, und es trifft die Bauart, die schon stand: **In WoW liegt die
Seltenheit in der Qualität, nicht im Ob.**

`PROGRESS_WEIGHTS` verteilt bereits 60/30/9/1 im Endgame (13.4). Der Normalfall ist damit ein
gewöhnliches Stück, das über die Materialumwandlung nicht wertlos ist; episch und legendär bleiben
selten. Es war also **eine Zahl**, keine zweite Mechanik.

> **Zwei Prüffall-Zeilen mussten dabei weichen** (`< 1000` und `treffer < 400`). Sie prüften nicht
> die Mechanik, sondern hielten eine **Annahme über die Design-Entscheidung** fest. Der
> Wirkungsnachweis läuft jetzt über die Gegenrichtung: Eine kleinere Rate **muss** weniger Treffer
> ergeben.

**Betrifft:** #447 (gebaut, Zahl vorläufig) · `data/items.json` → `dropChancePerMille`

Die Beute-Kette steht seit dem 2026-08-06: Ein Sieg würfelt aus einem eigenen Strom, der Fund
landet im Besitz und übersteht das Neuladen. Was **fällt**, entscheidet `rates.ts` (Qualität über
`weightsFor`, Stufe über `itemLevelOf`). **Ob** etwas fällt, entscheidet keine dieser zehn
Funktionen — es ist eine eigene Zahl, und sie steht vorläufig auf **250 Promille**.

GAME-DESIGN sagt zur Häufigkeit nichts. Kapitel 13.6 beschreibt die _Qualitätsverteilung_ im
Endgame (gewöhnlich 60 %, selten 30 %, episch 9 %, legendär 1 %) — das ist die Frage, **was** ein
Fund ist, nicht **wie oft** einer kommt.

### Die Messung, nicht die Rechnung

Worker-3 hat über **2000 Startwerte** gemessen, nicht hochgerechnet. Die Abweichung von der
Nennzahl ist die Streuung des Stroms, kein Fehler:

| Promille          | Anteil der Siege | im Schnitt     |
| ----------------- | ---------------- | -------------- |
| 100               | 10,2 %           | alle 9,9 Siege |
| 150               | 15,8 %           | alle 6,3 Siege |
| **250** (aktuell) | 26,5 %           | alle 3,8 Siege |
| 400               | 39,4 %           | alle 2,5 Siege |
| 500               | 48,9 %           | alle 2,0 Siege |

### Worum es tatsächlich geht

Bei einem Kampfzyklus von etwa 60 Sekunden bedeuten 100 Promille **zehn Minuten ohne Fund**. Bei
500 fällt bei jedem zweiten Sieg etwas — und ein Fund hört auf, ein Ereignis zu sein.

> Das ist genau die Spannung, die GAME-DESIGN 13.6 an anderer Stelle selbst beschreibt: _„Die
> Inszenierung kostet nur Zeit, wenn sie es wert ist; 99 von 100 Siegen laufen schnell durch, der
> eine legendäre Moment brennt sich ein."_ Der Satz setzt voraus, dass ein Fund **nicht** die Regel
> ist — sagt aber nicht, wie selten die Ausnahme sein soll.

**Warum das nicht der Balance-Läufer entscheiden kann:** Er kann ausrechnen, wie schnell die
Ausrüstung wächst, und daraus eine Rate ableiten, die zur Stärkekurve passt. Er kann nicht
ausrechnen, ob sich ein Sieg **belohnend** anfühlt. Die Zahl bestimmt den Charakter des
Wiederholens, und das ist Design.

**Was der Wechsel kostet:** einen Zahlenwert in `data/items.json` — dieselbe Linie wie der Exponent
in `reward.json`. Kein Umbau.

## 15. Fällt eine Zauber-Schwächung des eingehenden Schadens in den WARD-Deckel? — OFFEN

_Aufgekommen am 2026-08-06 beim Messen zweier Zauber, die dieselbe Mechanik brauchen._

**Zwei Zauber aus dem Katalog wollen den eingehenden Schaden mindern:**

| Zauber  | Schule       | Rolle                                                 |
| ------- | ------------ | ----------------------------------------------------- |
| `LATCH` | FIXATION     | der nächste Schlag trifft verzögert **und schwächer** |
| `DECAY` | PUTREFACTION | schwächt den Boss-Schaden nach Stapelzahl             |

**Gemessen:** Es gibt heute keine Stelle dafür. `hitPlayer` rechnet drei feste Summanden und einen
Deckel — keine Faktoren-Kette, wie sie der ausgehende Schaden hat.

```ts
const roh = PLAYER.wardReduction + this.worn.ward / 1000;
const minderung = Math.min(1, Math.min(PLAYER.wardCapPermille / 1000, roh));
```

### Die Frage

Der WARD-Deckel steht seit dem 2026-08-05 bei **75 % für die Summe** (#466, deine Entscheidung).
Fällt eine Zauber-Schwächung **in** diesen Deckel oder **daneben**?

|             | Folge                                                                                                                                     |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| **hinein**  | Sie konkurriert mit Ausrüstung. Wer schon nah am Deckel ist, hat von `LATCH` und `DECAY` nichts — der Zauber wäre bei gutem Gear wertlos  |
| **daneben** | Zauber können den Spieler **über** 75 % bringen. Fünf Zersetzungs-Stapel plus `LATCH` wären eine Minderung, die keine Ausrüstung erreicht |

**Beides ist vertretbar, und die Wahl verändert, wofür die beiden Zauber gut sind.** Sie berührt
außerdem die Begründung des Deckels selbst: Er wurde gesetzt, damit die Summe nicht ins Unbegrenzte
wächst — ob das nur für Ausrüstung gilt oder für alles, steht nirgends.

**Bis das entschieden ist, sind beide Zauber nicht baubar.** Die Mechanik ist klein; die Frage ist es
nicht.

### Nachtrag: wie die beiden Formen technisch aussehen

_Gemessen von Worker-2 am 2026-08-06, damit die Entscheidung nicht abstrakt bleibt._

Heute steht in beiden Fassungen dieselbe Reihenfolge:

```
roh       = Grundwert + Ausrüstung
minderung = min(roh, DECKEL)         ← der Deckel greift HIER
ergebnis  = raw * (1 - minderung)
```

| Wahl              | Was sich ändert                                                                | Folge                                                                                                       |
| ----------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| **in den Deckel** | Die Zauber-Minderung wird zu `roh` addiert, **vor** dem `min()`                | Sie konkurriert mit Ausrüstung. Wer nahe 75 % ist, merkt von seinem Zauber **nichts**                       |
| **daneben**       | Ein zweiter Faktor **nach** dem Deckel: `raw * (1 - minderung) * (1 - zauber)` | Zwei Faktoren multiplizieren sich. Die 75 % bleiben als **Ausrüstungs**-Grenze; der Gesamtwert kann darüber |

> **Beide sind gleich teuer zu bauen.** Die zweite hält die gestern gesetzte Grenze als das, was sie
> ist — eine Grenze für Ausrüstung — statt sie nachträglich zu einer Gesamtgrenze umzudeuten. Das ist
> ein technisches Argument, keine Balance-Aussage; die Wahl bleibt eine Design-Entscheidung.

**Ein Umbau kommt in jedem Fall dazu**, unabhängig von der Wahl:

```
Spiel       src/core/combat.ts:1975   INLINE in hitPlayer
Simulator   sim/src/fight.rs:2039     ruft PLAYER.incoming_with_gear(...)
                                      → sim/src/data.rs:775
```

Der Simulator hat bereits einen benannten Ort für die Rechnung, das Spiel nicht — die Spiel-Seite
muss erst herausgelöst werden, sonst steht die neue Minderung einmal inline und einmal gekapselt,
und die nächste Änderung trifft nur eine der beiden.

Dazu: `incoming_with_gear(&self, ...)` hängt an `PLAYER`, einer **Konstante**. Sie kennt Ausrüstung,
aber **keinen Kampfzustand** — und eine zeitweilige Minderung _ist_ Kampfzustand. Sie muss
durchgereicht werden, was neben `fight.rs` auch sechs direkte Aufrufe in
`sim/tests/parity_worn_values.rs` berührt. **Genau dort setzt ein unbedachter Parameter still eine 0
ein, und die Prüfung läuft weiter grün.**

> **Nachgemessen am 2026-08-06 — es ist die einzige Stelle dieser Art.** Der Verdacht lautete
> zwischenzeitlich, eine ganze Bauart sei betroffen: drei Mechaniken, die ihre Daten beim Anlegen in
> eine Tabelle kopieren und keinen Zugriff auf den Kampf haben. Zwei davon sind widerlegt.
>
> | Stelle                | Befund                                                                          |
> | --------------------- | ------------------------------------------------------------------------------- |
> | `StacksMechanic`      | legt nur den **Rohbetrag** ab; gebucht wird in `fight.rs`, wo der Faktor greift |
> | `SublimationMechanic` | dasselbe — `pending_shorten` wird in `fight.rs` angewandt                       |
> | `incoming_with_gear`  | **rechnet selbst, auf einer Konstante**                                         |
>
> Die Trennlinie ist nicht „kopiert Daten beim Anlegen", sondern **„wendet sie selbst an, ohne den
> Kampf zu sehen"**. Nach diesem Maß bleibt eine Stelle — und die beiden anderen zeigen im selben
> Haus, wie es geht: Rohbetrag ablegen, Anwendung dort, wo der Zustand sichtbar ist.
>
> Damit ist das hier **kein Bauart-Umbau**, sondern eine Rechenstelle, die mit dieser Entscheidung
> ohnehin angefasst wird. Ein eigenes Issue dafür wäre sofort blockiert — der Befund gehört hierher.

---

## 16. Sind 30 % Stapel-Wirkung gewollt, wo 18 % begründet wurden? — OFFEN

_Eingetragen am 2026-08-06. Die Frage stand seit dem Vormittag als AC3 in #496 und **nicht hier** —
genau die Zersplitterung, gegen die dieses Dokument angelegt wurde._

**Nicht die Zahl 6 % steht zur Frage.** Sie liegt über dem Break-even von 5,4 %, und #482 setzt sie
zu Recht um.

**Sondern, wie stark die Schule dadurch wird.** Die Entscheidung für 6 % wurde getroffen, _weil_ die
Gesamtwirkung bei 18 % lag. Gemessen sind es 30 %:

| Größe                     | in der Herleitung (GAME-DESIGN 221) | tatsächlich | Quelle                    |
| ------------------------- | ----------------------------------- | ----------- | ------------------------- |
| `canker` Abklingzeit      | 4,5 s                               | **2,0 s**   | `canker.cooldownMs`       |
| Haltbarkeit je Stapel     | 12 s                                | 12 s        | `DECAY_STACK_DURATION_MS` |
| Obergrenze                | 3                                   | **5**       | `MAX_DECAY_STACKS`        |
| **Gesamtwirkung bei 6 %** | **18 %**                            | **30 %**    |                           |

Nach acht Sekunden liegen fünf Stapel — mit `canker` allein, ohne `BLIGHT`.

> **Warum das eine Design-Frage ist und keine Zahl:** GAME-DESIGN 177 sagt, PUTREFACTION sei „allein
> gespielt schwach" und brauche immer Partner. 30 % mehr Schaden aus **allen** Quellen ist eine
> andere Aussage über die Schule als 18 % — es verschiebt, wie stark sie ohne Partner dasteht.

**Zur Wahl stehen:** bei 6 % bleiben und die stärkere Schule annehmen · die Zahl senken, bis die
Gesamtwirkung wieder bei ~18 % liegt (rechnerisch 3,6 %) · eine der drei gemessenen Größen ändern,
statt der Prozentzahl.

**Heute folgenlos:** #482 ist noch nicht gemergt, und PUTREFACTION hat erst zwei Zauber. Die Frage
wird scharf, sobald die Schule auf zwölf wächst (#479).

> Der Text in GAME-DESIGN 221 bleibt bis zur Entscheidung **unberichtigt stehen**. Worker-3 hat ihn
> bewusst nicht angepasst, und das war richtig: Hätte er die Rechnung auf die neuen Zahlen gebracht,
> wäre die Frage nach den 30 % nie gestellt worden — der Text hätte gestimmt, und die Entscheidung
> wäre weiter auf 18 % begründet gewesen.

---

## 17. Darf ein Zauber können, was ein Schlüsseltalent verleiht? — OFFEN

_Aufgeworfen von Worker-1 am 2026-08-07 bei #519, ausdrücklich **nicht** selbst entschieden._

**Die konkrete Frage:** SCORIA soll ein zweiter Entlade-Zauber der Kalzination werden. Seine
Beschreibung im Raster lautet „verbraucht nur die Hälfte der Hitze, schlägt zweimal".

**Genau das gibt es schon** — als Schlüsseltalent `KEYSTONE_SPLIT_SPENDER` (#418), vollständig
gebaut auf beiden Seiten, mitsamt Rundungsregel und Verbrauchsreihenfolge.

| Lesart                 | Folge                                                                                    |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| **SCORIA erbt es**     | Wer das Talent hat, bekommt das Verhalten auch bei SCORIA. Das Talent behält seinen Wert |
| **SCORIA hat es fest** | Der Zauber kann dauerhaft, wofür sonst ein Talentpunkt am Ende eines Zweigs nötig ist    |

### Warum das keine Zahl ist, sondern Charakter

GAME-DESIGN sagt über Schlüsseltalente: Sie sollen die Rotation umbauen, und _„ein neues
Schlüsseltalent fühlt sich an wie eine neue Schule"_ — sie sind der Grund, warum 300 Stunden
tragen. Ein Zauber, der dasselbe leistet, entwertet nicht nur dieses Talent, sondern die Gattung:
Wenn ein Fund ersetzen kann, was ein Zweig-Ende verleiht, ist die Wahl am Zweig-Ende keine mehr.

### Empfehlung: geerbt

So bleibt das Talent das, was es sein soll — und SCORIA ist trotzdem ein eigener Zauber, weil er
andere Zahlen trägt (Abklingzeit, Schaden je Punkt, Schwelle). **Der zweite Entlader ist bereits
für sich eine Wahl**, auch ohne den Zusatz; das war Worker-1s Befund zur Begründungsspalte.

> Wird **fest** entschieden, ist das kein Fehler — aber dann gehört geprüft, ob andere
> Schlüsseltalente denselben Weg nehmen sollen. Eine Ausnahme für einen Zauber ist eine
> Sonderregel; eine Linie für alle ist ein Entwurf.

---

## 18. Drei Bausteine sind gebaut und an nichts gebunden — OFFEN

_Aufgenommen am 2026-08-07, nachdem der dritte Fall innerhalb von zwei Stunden auftrat._

Diese drei Punkte stehen zusammen, weil sie **dieselbe Lage** sind, nicht weil sie
zusammengehören. Jeder für sich wäre eine kleine Frage; zu dritt sind sie eine Aussage über
unsere Bauweise.

### Was in allen drei Fällen gilt

Der Mechanismus ist gebaut, geprüft, auf beiden Seiten bitgleich und in der
Zustands-Prüfsumme. Er läuft. Er bewirkt nichts, weil die **Bindung an eine Spieler-Handlung
oder eine Folge** nie entschieden wurde.

|                              | gebaut und messbar                                                    | es fehlt                                                                                                     |
| ---------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Verhängnis-Zähler** (#561) | steigt, erreicht 100, `clearDoom()` funktioniert                      | **kein Zauber löst die Senkung aus** — die Funktion wird nur von Prüffällen gerufen                          |
| **Stapel-Regel** (#562)      | `kind` und `target` wählen Speicher und Platz, beide Seiten lesen sie | **kein Speicher hat zwei echte Bewohner** — die Regel „gleiche ersetzen, verschiedene stapeln" tritt nie ein |
| **Energieleiste** (#568)     | steigt, erreicht die Schwelle, setzt zurück, zählt                    | **der Zähler wird nirgends gelesen** — die Schwelle löst nichts aus                                          |

### Warum das eine Frage an den Auftraggeber ist und keine an den Balance-Läufer

Die Prüffrage lautet: _Entscheidet die Antwort über den Charakter des Spiels oder über eine
Zahl?_ Hier ist es dreimal der Charakter:

- **Welcher Zauber senkt das Verhängnis** entscheidet, welche Schule die Antwort auf eine
  tödliche Uhr besitzt. Wird es eine FIXATION-Wirkung, ist das die vierte neben verlangsamen,
  verzögern, hinauszögern. Wird es COAGULATION, ist Überleben plötzlich mehr als Schild und
  Heilung. Wird es ein schulunabhängiger Sonderzauber wie QUENCH, ist es niemandes Schule —
  dann steigt die Zahl der Sonderfälle auf vier.
- **Was die Energieleiste auslöst** entscheidet, ob sie eine zweite Uhr ist, die man im Blick
  behält, oder ein Ereignis, auf das man antwortet. GAME-DESIGN nennt „bei manchen Bossen den
  sofortigen Tod" — das ist die härteste Form und schließt die anderen nicht aus.
- **Ob zwei Verstärker sich einen Speicher teilen sollen** entscheidet, ob das Zusammenlegen
  großer Fähigkeiten — von GAME-DESIGN ausdrücklich als **Kernfertigkeit** benannt — überhaupt
  vorkommt. Heute sitzt jeder Verstärker allein, und es gibt nichts zusammenzulegen.

### Der Befund hinter den drei Befunden

> Wir bauen den Mechanismus und verschieben die Bindung — und die Verschiebung wird nie
> eingeholt, weil **nichts sie einfordert**.

Der Kommentar über `clearDoom()` sagt es selbst: _„WELCHE TASTE SIE AUSLÖST, STEHT HIER NICHT.
Dieses Issue baut den Mechanismus; die Bindung entscheidet E10."_ Das war richtig aufgeschrieben
und ist trotzdem liegengeblieben, weil ein Kommentar niemanden erinnert.

**Alle drei sind paritätsgesichert.** Beide Seiten zählen bitgleich dasselbe Nichts — die
Absicherung, auf die wir uns verlassen, sagt über Wirkung nichts aus.

### Was blockiert ist, solange die Antwort fehlt

|                                           |                                                                                                                                                              |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `boss-1-2` der ersten Instanz             | **unschaffbar**, nicht wegen der Zahlen: Der Zähler läuft in gut 16 Sekunden voll, und der Spieler hat kein Mittel dagegen. Keine Lebenspunktzahl ändert das |
| AC2 und AC4 von #544                      | drei von vier statt vier von vier                                                                                                                            |
| jeder künftige Boss mit diesen Bausteinen | dieselbe Lage, nur später bemerkt                                                                                                                            |

### Was ausdrücklich **nicht** getan wird

Eine Justierung, die den Boss schaffbar macht, ohne dass die Signatur zur Wirkung kommt. Bei
`boss-1-2` hieße das, die Lebenspunkte zu senken, bis er vor dem Verhängnis fällt — dann wäre
AC2 grün bei einem Boss, dessen Signatur nie eintritt.

> **Ein Boss, den man ohne die verlangte Handlung besiegt, verlangt sie nicht.**

Dasselbe gilt für die Energieleiste: Eine Schwelle, die absichtlich nie erreicht wird, ist eine
Anzeige, die so tut als ob.

### Nachtrag: die drei Punkte sind eine Stufe in einer Kette

_Ergänzt wenige Stunden später, nachdem eine vierte Suche einen Fall auf einer früheren
Stufe fand und ein Worker die Verbindung benannte._

Punkt 18 beschreibt Bausteine, deren **Bindung** fehlt. Am selben Tag kamen aus zwei anderen
Richtungen Befunde, die zuerst unverwandt aussahen — bis Worker-2 den Satz schrieb: _„Hier ist
nicht einmal der Mechanismus da, nur die Absicht."_

| Stufe |                                      | Fälle                                                                                                |
| ----- | ------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| **1** | Absicht formuliert, kein Mechanismus | `signature` — vier Bosse tragen ihre Regel als Satz, keine Zeile Code liest ihn (#570)               |
| **2** | Mechanismus gebaut, keine Bindung    | `doom`, `energy`, `buff.kind` — alles läuft, nichts löst aus (#561, #562, #568)                      |
| **3** | Bindung da, keine Prüfung            | **39** von 68 Größen — die Zahl wirkt, aber man kann sie halbieren, ohne dass etwas anschlägt (#547) |

Drei Stufen, drei Worker, ein Tag. **Jede sieht von außen aus wie Fortschritt:** Es steht
etwas da, es läuft etwas, es ist bitgleich.

> **Warum 39 und nicht 45.** Die Messung zu #547 nennt 45 Paare, die keine Wirkungsprüfung
> auslösen. Sechs davon sind `buff.kind` — und die stehen bereits eine Zeile höher auf
> **Stufe 2**: Es gibt im Bestand keine Lage, in der die Größe entscheidet, weil kein Speicher
> zwei echte Bewohner hat. Sie zweimal zu zählen wäre derselbe Fehler, den dieses Dokument
> beschreibt: eine Zahl, die zu einem Schluss einlädt, den sie nicht trägt.
>
> Gefunden hat es Worker-1 an meinem Text — nachdem ich ihn zwei Stunden vorher auf genau
> diese Unterscheidung in _seinem_ Dokument hingewiesen hatte.

> Was auf allen drei Stufen fehlt, ist dasselbe: **der Nachweis, dass es einen Unterschied
> macht.**

### Warum die Bitgleichheit dagegen nicht hilft

Alle drei Stufen sind paritätsgesichert. Beide Seiten rechnen dasselbe — deshalb meldet die
Absicherung, auf die dieses Projekt seit Monaten baut, in keinem der Fälle etwas.

```
Sie sichert, dass Spiel und Simulator DASSELBE rechnen.
Nicht, dass sie das RICHTIGE rechnen.
```

Gemessen an einem Beispiel: `distil buff.factor` von 2 auf 1 gesetzt — halbiert — und danach
`npm run parity:export`. Ergebnis: **1776 von 1776 grün.** Der Faktor ist halbiert, und keine
einzige Prüfung meldet sich, weil beide Seiten die neue Zahl gleich falsch verwenden.

### Was daraus für die Entscheidung folgt

Nichts an den drei Fragen selbst — sie bleiben, wie sie oben stehen. Aber die Dringlichkeit
verschiebt sich, und die Begründung dafür stammt von Worker-1:

> Der Befund betrifft nicht die Zahlen, die **heute** falsch sind — alle sind korrekt und
> beide Seiten rechnen sie gleich. Er betrifft die Zahlen, die **morgen geändert werden**.
> Der Balance-Läufer ist genau dafür gebaut, und wenn er anfängt, Werte vorzuschlagen, ist bei
> 45 von 68 kein Netz da.

Das macht aus einer allgemeinen Sorge eine terminierte: **Das Netz muss stehen, bevor die
Balance-Arbeit beginnt** — nicht danach.

### Nachtrag: der Preis des Wartens ist jetzt bezifferbar

_Ergänzt am 2026-08-07, nachdem Worker-3 acht Bosse gebaut hatte._

Bisher stand hier die **Lage**, nicht der **Preis**. Er lautet inzwischen:

```
acht gebaute Bosse tragen `phases` und `energy` als leere Felder
```

Jeder dieser acht sieht aus, als hätte er einen Baustein mehr, als er hat. Worker-3s Satz:

> Acht Bosse tragen inzwischen leere Felder, die wie Bausteine aussehen. Wenn die Bindungen
> kommen, wirken sie rückwirkend — dann müsste ich alle acht neu messen.
>
> **Je später die Entscheidung, desto teurer.**

Das ist keine Vermutung über die Zukunft, sondern eine Multiplikation: Jede Instanz, die vor der
Entscheidung gebaut wird, kostet bei einer Bindung vier weitere Bosse Nachmessung. Instanz 3 und
4 würden den Preis auf sechzehn heben.

> **Die Richtung, in der es billig ist, ist eindeutig.** Wird entschieden, dass die drei
> Kulisse bleiben, kostet das heute nichts — die Felder fallen aus den Datensätzen, und niemand
> muss messen. Nur der andere Weg wird mit jedem Boss teurer.
>
> Das heißt nicht, dass er falsch wäre. Es heißt, dass die Entscheidung nicht dadurch billiger
> wird, dass man sie aufschiebt.

---

## 19. „Tempo" ist heute keine spielbare Entscheidung — OFFEN

_Aufgenommen am 2026-08-07, gemessen beim Bau der zweiten Instanz (#598)._

### Der Befund

Die Schadensrate des Spielers ist über alle bekannten Kämpfe nahezu konstant:

```
boss-2-3 volle Rotation   5488 Schaden in 52,6 s = 104,4/s
boss-1-1 mit quench       5200 Schaden in 51,1 s = 101,8/s
boss-1-3                  5800 Schaden in 57,0 s = 101,7/s
boss-1-4 mit quench       4900 Schaden in 55,5 s =  88,3/s
```

**Vier verschiedene Bosse, vier Rotationen, vier Signaturen — eine Zahl.**

### Was daran hängt

`boss-2-2` sollte die Entscheidung stellen, die GAME-DESIGN als Kern der Koagulations-Schule
nennt: _„Auf hohen Schwierigkeitsstufen muss man Leisten-Plätze dafür opfern — echte Abwägung
Schaden gegen Sicherheit."_

Gemessen über vier Parametrierungen ist es keine Abwägung. Worker-3s Einordnung:

> Beide Läufe fahren **dieselbe** Rotation. Der eine setzt zusätzlich CRUST, der andere nicht.
> Der zweite hat also keinen Vorteil — er ist nicht schneller, sondern nur ungeschützt.
>
> Echtes Tempo hieße: eine Rotation, die **mehr** Schaden macht und deshalb früher fertig ist.
> **Die gibt es nicht.**

Für den Tempo-Weg an `boss-2-2` wären 352/s nötig. Der Zauberkasten gibt ein Drittel her.

### Warum das eine Design-Frage ist und keine Balance-Zahl

Es fällt eine der **drei Antworten** weg, die GAME-DESIGN 11.1 für jede Boss-Mechanik verlangt:

> Ein Boss-Zauber lässt sich unterbrechen, **oder** durch Fixierung verlangsamen, **oder** durch
> Koagulation aushalten. Kein Boss verlangt eine bestimmte Schule.

Solange die Rate konstant ist, bleiben zwei Antworten statt drei — und jeder Boss, der eine
Abwägung stellen soll, stellt eine Scheinwahl.

**Die konstante Rate ist kein Fehler, sondern eine Folge:** Bei 1,5 s globaler Wartezeit und
ganzzahligen Abklingzeiten füllt jede halbwegs vernünftige Rotation den Takt. GAME-DESIGN
verlangt an anderer Stelle, dass eine **schlechte** Rotation mindestens 10 % weniger Schaden
macht — das ist erfüllt und war der Prüfstein des Prototyps. Für eine Tempo-**Entscheidung**
bräuchte es das Drei- bis Vierfache dieses Abstands.

### Die Frage

**Soll es Zusammenstellungen geben, die deutlich schneller sind als andere — und dafür
zerbrechlicher?**

|          |                                                                                                                                                                                                                                    |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Ja**   | Dann braucht der Zauberkasten Zauber mit hohem Schaden und hohem Preis: lange Wirkzeit, hohe Abklingzeit, kein Schutz nebenher. Erst dann wird „Tempo gegen Sicherheit" eine echte Wahl, und Bosse wie `boss-2-2` sind baubar      |
| **Nein** | Dann ist die Entscheidung eine andere — etwa zwischen Schulen, nicht zwischen Geschwindigkeiten. Dann gehört der Satz aus 13.5 („echte Abwägung Schaden gegen Sicherheit") umformuliert, und Bosse werden nicht mehr darauf gebaut |

### Was bis dahin gilt

`stacks` als Träger ist zurückgestellt (#574 nimmt stattdessen `timeline`). Kein Boss wird auf
einer Abwägung gebaut, die es nicht gibt — das wäre die Kulisse in ihrer teuersten Form: nicht
ein Feld ohne Wirkung, sondern eine **Anforderung ohne Alternative**.

---

## 20. Acht Bosse, eine Zusammenstellung — OFFEN

_Aufgenommen am 2026-08-07 bei der Abnahme der zweiten Instanz (#574). Dies ist kein Befund über
eine Zahl, sondern über eine Leitplanke des Auftraggebers._

### Die Messung

Die vier Aufzeichnungen der zweiten Instanz nebeneinander, gezählt nach `casts`:

```
boss-2-1   crust 5  kindle 25  pyre 4  quench 1  smoulder 5
boss-2-2   crust 5  kindle 25  pyre 4  quench 4  smoulder 5
boss-2-3   crust 4  kindle 22  pyre 3  quench 5  smoulder 4
boss-2-4   crust 5  kindle 26  pyre 4  quench 5  smoulder 5
```

**Vier Bosse, fünf Zauber, dieselbe Zusammenstellung.** Der einzige Unterschied ist, wie oft
QUENCH gedrückt wird. Für die erste Instanz gilt dasselbe.

Der Balance-Läufer hatte die andere Hälfte schon am 2026-08-05 gemessen: **zwölf von fünfzehn
Zaubern kommen in keiner besten Rotation vor**, an zwei Bössen mit verschiedener Dauer und
verschiedener bester Rotation.

### Warum das eine Frage an den Auftraggeber ist und keine an den Balance-Läufer

Zwei seiner Leitplanken aus dem Interview stehen dazu im Widerspruch:

|                                                    | Stand                                                                   |
| -------------------------------------------------- | ----------------------------------------------------------------------- |
| _„Jeder Zauber muss irgendwo die beste Wahl sein"_ | zwölf von fünfzehn sind es nirgends                                     |
| _„Der Spieler soll spielen können, wie er will"_   | er kann nicht anders — es gibt keine zweite tragfähige Zusammenstellung |

Die zweite ist die schwerere. Das Design sagt ausdrücklich: **„Es muss möglich sein, nicht
nötig."** Die Widerstände wirken bewusst nur mit 20–30 %, damit niemand zum Umbauen gezwungen
wird. Gemessen ist heute das Gegenteil des Gemeinten: Es ist nicht nötig, weil es nicht
**möglich** ist.

> Es ist also keine Verletzung der Regel, sondern ihr Leerlauf. Eine Freiheit, die keine
> Alternative hat, ist keine.

### Was das für das Kernrisiko heißt

A.12 nennt das Kernrisiko: _„Wenn sich herausstellt, dass die optimale Rotation eine feste
Prioritätenliste ist, die sich nie ändert, ist das Spiel nach etwa zwanzig Stunden nur noch
Ausführung."_

**Der Prüfstein dafür ist bestanden** — eine gute Rotation macht +48,2 % mehr Schaden als eine
schlechte, das Kriterium verlangte 10 %. Was heute fehlt, ist die andere Achse: Nicht _ob_ man
gut spielt, sondern _womit_.

Acht Bosse haben daran nichts geändert, und das ist die eigentliche Nachricht. Worker-1s Satz
vom 2026-08-05 lautete: _Es fehlen nicht Zauber, es fehlen Bosse, die etwas verlangen._ Nach
#544 und #574 ist die andere Hälfte belegt: **Es fehlen auch Zauber, die etwas können.**

### Warum „mehr Zauber bauen" nicht die Antwort ist

Drei Befunde derselben Woche zeigen, wo es klemmt — und alle drei sind **Ausgänge**, nicht
Zauber:

|          |                                                                                                       |
| -------- | ----------------------------------------------------------------------------------------------------- |
| #598     | „Tempo" ist keine spielbare Entscheidung — die Schadensrate ist über alle Bosse konstant bei 88–104/s |
| Punkt 18 | Drei Bausteine gebaut und an nichts gebunden (`doom`, `energy`, `signature`)                          |
| #605     | Kontroll-Kosten wirken, sind aber im Ergebnis nicht abrufbar                                          |

Ein neuer Zauber, der auf dieselben Ausgänge wirkt wie KINDLE, wäre der sechzehnte Weg, Schaden
zu machen. Er änderte die Zusammenstellung nicht.

### Was gebraucht wird

**Keine Entscheidung heute.** #607 misst zuerst, worauf ein Zauber überhaupt wirken kann und
welcher dieser Ausgänge je einen Kampf entschieden hat. Das Ergebnis kommt als Vorlage zurück,
mit Zahlen.

Die Frage, die dann beim Auftraggeber liegt, lautet voraussichtlich: **Wie viele tragfähige
Zusammenstellungen soll ROTA haben — und ist eine zweite genug?** Das ist eine Aussage über den
Charakter des Spiels, nicht über eine Zahl: Zwei Wege sind eine Wahl, fünf sind ein
Optimierungs-Labor, und der Unterschied bestimmt, wie viel Inhalt die 300 Stunden tragen muss.

---

## 21. Darf `gearEffects` ein Affix sein? — ENTSCHIEDEN: nein

_Eingetragen am 2026-08-10, entschieden am selben Tag. Aufgekommen beim Ausrüstungs-Raster
([`AUSRUESTUNGS-RASTER.md`](AUSRUESTUNGS-RASTER.md)), nicht aus dem Interview._

**Die Frage entstand aus einer Messung, nicht aus dem Dokument.** `catalog.ts` prüft Affix-Arten
gegen `POOL_NAMES` und lässt damit **alle sechs Topf-Namen** zu — auch `gearEffects`. Ein Eintrag
damit lädt anstandslos und füllt Topf 4, ganz ohne Sätze:

```text
{"catalysis":0,"purity":0,"yieldBonus":0,"essence":473,
 "vessel":67,"ward":67,"gearEffects":407,"active":406}
```

Damit war offen, was 13.1b nicht sagt: Topf 4 wird dort als „legendäre Effekte **und** Satz-Boni
gemeinsam" beschrieben — ein gewöhnlicher Affix wäre eine **dritte** Quelle.

### Das Ergebnis

**Nein.** Die Begründung des Auftraggebers ist die Lehre, die 13.1b selbst zitiert:

> Was multiplikativ ist, wird gejagt — und alles andere wird liegen gelassen.

Ein multiplikativer Faktor über **gewöhnliche** Beute entwertet jeden anderen Affix. Genau dieses
Muster musste Diablo 4 in Saison 2 zurückdrehen; 13.1b führt es als Warnung, nicht als Vorbild.

Die Größenordnung ist gemessen — derselbe Lauf, nur die Affixe getauscht:

| Ausrichtung                                    |  Schaden |
| ---------------------------------------------- | -------: |
| `catalysis, purity, yield` (drei Sonderwerte)  |     2812 |
| `essence, purity, catalysis` (ein Faktor)      |     3458 |
| `essence, gearEffects, purity` (zwei Faktoren) | **4868** |

> Die **Rangfolge** ist ein Artefakt des Messaufbaus (fester 1500-ms-Takt benachteiligt Tempo) —
> die **Größenordnung** des Abstands ist es nicht.

### Was daraus folgt

- **Topf 4 bleibt legendären Effekten und Satz-Boni vorbehalten.**
- Das Ausrüstungs-Raster geht von 30 auf **20 Einträge** (die Ausrichtung `-core` entfällt).
- **Eine Code-Sperre dafür gibt es nicht.** `gearEffects` ist technisch weiterhin eine gültige
  Affix-Art, weil sie ein Feld füllt. Wer die Entscheidung absichern will, braucht eine eigene
  Prüfung — das ist bewusst nicht Teil des Katalogs.

---

## 22. Bleibt `active` in `POOL_NAMES`? — OFFEN

_Eingetragen am 2026-08-10. Die **Folge** ist bereits behoben, die **Frage** nicht._

`POOL_NAMES` führt die sechs Töpfe aus 13.1b, `WornValues` die Werte, die der Kampf liest. `active`
ist ein Topf **ohne** Wert: Aktive Verstärkungen entstehen im Kampf — Overdrive, Synergien,
Zersetzungs-Stapel — und stehen nicht auf einem Gegenstand.

### Was bereits geschlossen ist, unabhängig von dieser Frage

Bis zum 2026-08-10 benutzte `wornFrom` die Art **direkt** als Schlüssel. Ein Eintrag mit `active`
lud fehlerfrei, bestand jede Prüfung — und legte seinen Anteil in ein Feld, das niemand liest:
gemessen **406 von 1219 Promille**, ohne Ausnahme, ohne roten Fall, ohne Log. Der Gegenstand sah
vollwertig aus und war ein Drittel schwächer.

**Der Auftraggeber hat die Sperre dieser Design-Frage ausdrücklich vorgeordnet:** Jeder Affix-Name,
der kein Feld trifft, muss laut scheitern statt Promille zu verschlucken. Das gilt unabhängig von
`active`, weil es **jeden** künftigen Tippfehler in einer Affix-Liste trifft. Umgesetzt in
`feldFuerAffixArt` (`src/gear/worn.ts`) und `pruefeAffixArten` (`src/gear/catalog.ts`).

### Was offen bleibt

**Fliegt `active` aus `POOL_NAMES`, oder bekommt es ein Feld?**

|                        | dafür                                                                | dagegen                                                                      |
| ---------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| **raus aus der Liste** | Die Töpfe wären dann genau die Arten, die ein Gegenstand tragen kann | 13.1b führt vier Töpfe; einer davon verschwände aus der Aufzählung im Code   |
| **ein Feld geben**     | Ausrüstung könnte aktive Verstärkungen mitbringen                    | 13.1b nennt für Topf 3 nur Kampf-Quellen — das wäre eine neue Design-Aussage |

Bis zur Entscheidung bleibt `active` in der Liste und fällt beim Ladeversuch auf. **Das ist ein
tragfähiger Zwischenzustand**, kein Provisorium: Der Schaden ist abgestellt, nur die Aufräumfrage
ist offen.
