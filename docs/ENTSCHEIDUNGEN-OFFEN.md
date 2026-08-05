# Was beim Auftraggeber liegt

Fünf Entscheidungen, die niemand sonst treffen darf, weil sie das Spiel betreffen und nicht seine
Umsetzung. Jede steht hier mit **Beleg, Optionen und Folge** — und mit dem, was sie blockiert.

> **Diese Datei ist kein Wunschzettel.** Was hier steht, hält Arbeit auf: zwei Issues sind
> vollständig blockiert, zwei weitere lassen sich nicht abnehmen, ohne die Frage vorher zu
> beantworten. Ist eine Entscheidung getroffen, wandert sie ins GAME-DESIGN und verschwindet hier.

Stand: 2026-08-05. Gemessen, nicht erinnert — jeder Beleg unten ist ein ausgeführter Befehl.

---

## 1. Darf derselbe Zauber auf zwei Plätzen liegen?

**Blockiert:** #287 (AC1 verlangt die Antwort, bevor gebaut wird)

Die Leiste hat 12 bis 16 Plätze bei 60 Zaubern. Ob **derselbe** Zauber zweimal liegen darf, steht
nirgends — GAME-DESIGN 714 sagt nur, dass der Spieler bestimmt, welcher Zauber auf welchem Platz
liegt.

```
$ grep -rn "doppelt|bereits belegt|duplicate" src/progression/ src/core/loadout.ts
(keine Sperre gefunden)
```

Heute ist es also erlaubt, und die Vorschau zählt es als zwei Einzahler:

```
{ einmal: 1, zweimal: 2 }      derselbe Zauber, ein Platz gegen zwei
```

|              | Argument                                                                               | Folge für die Umsetzung                                                                                            |
| ------------ | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **erlaubt**  | Ein zweiter Knopf für denselben Zauber kann bequemer unter einem anderen Finger liegen | Die **Zählung** wird repariert: ein Zauber = ein Einzahler                                                         |
| **verboten** | Ein Platz ist knapp; ihn zu verdoppeln verschenkt ihn                                  | Die **Auswahl** sperrt ihn, und die Zählung stimmt von selbst. Alte Spielstände müssen beim Laden bereinigt werden |

**Warum es zählt:** GAME-DESIGN 16 verlangt von der Vorschau die Aussage, _wie viele Zauber je
Schul-Mechanik einzahlen_ — „man erkennt sofort, ob die Hitze-Mechanik mit nur einem Zauber trägt".
Zwei Plätze mit demselben Zauber sind keine zwei Einzahler. Die Vorschau beantwortet heute genau die
Frage falsch, für die es sie gibt.

---

## 2. Wie verläuft die Punktekurve?

**Blockiert:** #276 · **hält auf:** E5 insgesamt, weil Freischaltungen und Aufwertung daran hängen

Es gibt keine Punkte. `score` kommt im Quelltext an genau zwei Stellen vor:

```
src/render/overlay.ts:352        set('score', '—');
src/analysis/save-format.ts:178  /score(?!board)/i    ← verbotenes Feld im Spielstand
```

Der zweite ist richtig so: GAME-DESIGN 18.5 verbietet berechnete Werte im Spielstand, die auf
kommende Kämpfe zurückwirken. Es fehlt also nicht die Speicherung, sondern die **Formel**.

Das Design gibt die Richtung vor und lässt die Kurve offen:

```
docs/GAME-DESIGN.md:2179
| Punkteformel | Konkrete überproportionale Kurve | E5 |
```

**Überproportional** heißt: Die letzten Sekunden Restzeit sind unverhältnismäßig viel wert, damit
Feinoptimierung sich lohnt. Was fehlt, ist wie stark.

| Kurve            | Verhalten                                 | Wem sie dient                                 |
| ---------------- | ----------------------------------------- | --------------------------------------------- |
| **quadratisch**  | doppelte Restzeit, vierfache Punkte       | ausgewogen; die üblichen Verdächtigen         |
| **kubisch**      | doppelte Restzeit, achtfache Punkte       | den Könnern; Gelegenheitsspieler sammeln kaum |
| **exponentiell** | jede Sekunde ist mehr wert als die vorige | dem Endgame, mit Zahlen-Inflation als Preis   |

**Die Entscheidung hat eine Nebenwirkung, die leicht übersehen wird:** Die Punkte sind zugleich das
Sicherheitsventil. GAME-DESIGN führt Punkte-Schwellen als den Weg, der einem feststeckenden Spieler
offen bleibt — eine Kurve, die nur Könnern etwas auszahlt, verschließt genau denen das Ventil, die
es brauchen.

---

## 3. Bleibt `DECAY_STACK_PERCENT` bei 3?

**Hält auf:** nichts unmittelbar — aber jede Balance-Aussage zu PUTREFACTION steht darauf

Befund aus #224, gemessen an beiden Seiten:

```
sim/src/fight.rs:168      pub const DECAY_STACK_PERCENT: u32 = 3;
src/core/combat.ts:101    export const DECAY_STACK_PERCENT = 3;
```

Bei 4,5 s Abklingzeit und 12 s Haltbarkeit sind höchstens **drei** Stapel gleichzeitig erreichbar.
Drei Stapel à 3 % sind 9 % mehr Schaden von allen Quellen — und der Break-even, ab dem sich der
Platz auf der Leiste überhaupt lohnt, läge bei **5,4 % je Stapel**.

**Wenn der Wert bleibt**, ist PUTREFACTION allein nicht spielbar. Das kann gewollt sein — GAME-DESIGN
sagt ausdrücklich „allein gespielt schwach, braucht immer Partner". Die Frage ist, ob 9 % gegen einen
verschenkten Leisten-Platz noch „schwach" heißt oder schon „nie die beste Wahl", und das zweite
verstößt gegen den Grundsatz, dass jeder Zauber irgendwo die beste Wahl sein muss.

---

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

## 5. Bleibt PUTREFACTION hell-violett statt schwarz-violett?

**Hält auf:** nichts — die Abweichung ist gebaut und dokumentiert, sie braucht nur ein Ja oder Nein

Das Design sagt:

```
docs/GAME-DESIGN.md:789   | PUTREFACTION | schwarz-violett |
docs/GAME-DESIGN.md:791   > Schwarz-violett braucht auf dunklem Grund sorgfältige Abstimmung.
```

Gemessen ist es kein Abstimmungsproblem, sondern ein Zielkonflikt. Kontrast gegen den Grund des
Player Home (`#1a1613`):

```
#5b3a8e 2,09 · #6d46a5 2,63 · #7a52b5 3,17 · #8d5fc4 3,91 · #9b6fd0 4,78
```

Alles unter etwa `#9060c8` ist als Text unlesbar. Gebaut ist `#9b6fd0` — **hell**-violett, und damit
nicht die Design-Farbe. Die vier anderen Schulen liegen bewusst im selben Korridor (5,00 · 6,43 ·
6,21 · 5,14), damit keine optisch dominiert.

|                          | Folge                                                                                                                               |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **hell-violett bleibt**  | GAME-DESIGN 789 wird auf den gebauten Wert nachgezogen. Kein Code ändert sich                                                       |
| **schwarz-violett gilt** | Der **Grund** an dieser Stelle wird heller, nicht die Schrift dunkler. Das berührt das Player Home als Ganzes, nicht nur eine Farbe |

---

## Was hier bewusst **nicht** steht

- **Die Gestaltung der sechs neuen Anzeigen aus #294** — das ist Umsetzung und gehört dem Bearbeiter.
- **KINDLEs Abklingzeit** — `cooldownMs: 1500` ist der GCD und damit keine offene Frage, sondern die
  Messlatte für jeden weiteren Kalzinations-Zauber (#268).
- **Alles aus GAME-DESIGN 21** — dort steht, was bereits verworfen wurde. Nichts davon wird ohne
  ausdrückliche Rücksprache wieder aufgemacht.
