# Vier Bosse, eine Zusammenstellung — nachgemessen (#607)

**Stand 2026-08-14.** Alle Zahlen aus eingecheckten Läufern, mit ihrem Aufruf
daneben. Der Simulator ist deterministisch: Streuung 0,0 in jedem Lauf, kein
Rauschanteil.

> **Die Zahlen im Issue sind vom 07.08. und dreifach überholt.** Es sind nicht
> 17 spielbare Zauber, sondern 58; nicht „zwölf von fünfzehn" tot, sondern 37
> von 58; und die Messung dort lief bei einer Suchlänge, die keinen einzigen
> Boss ausreizte.

## AC1 — welche Zauber in mindestens einer besten Rotation vorkommen

```
cargo run --release --example rotation-search -- \
  --bosses boss-1-1,…,boss-4-4,first-crucible --max-len N
```

17 echte Bosse (ohne `test-*`), 58 Zauber mit `unlockLevel <= 100`.

| `--max-len` | gewirkt |    tot | Abbruchgrund                      |
| ----------: | ------: | -----: | --------------------------------- |
|           6 |      15 |     43 | 17× LengthLimit                   |
|           8 |      18 |     40 | 12× LengthLimit, 5× NoImprovement |
|          12 |  **21** | **37** | 3× LengthLimit, 14× NoImprovement |

**Jede Zahl steht mit ihrem Abbruchgrund**, weil sie ohne ihn nichts sagt: Die
15 bei Länge 6 gehört zu einer Suche, die bei _jedem_ Boss an ihre Grenze
stieß.

**Kein Zauber fällt in irgendeinem Schritt weg.** Diese Monotonie ist der
Grund, warum „Obergrenze" überhaupt eine zulässige Aussage ist — ohne sie wäre
jede Zahl von jeder Länge abhängig und keine mit einer anderen vergleichbar.

### Die 37 ist eine Obergrenze, keine Endzahl

Drei von siebzehn Bossen stoßen bei Länge 12 weiter an `LengthLimit`. Ein
16er-Lauf wurde trotzdem **nicht** gefahren, und der Grund ist ein Sachgrund
statt eines Aufwandsarguments:

> Die drei offenen Bosse können höchstens Zauber beitragen, die bei den anderen
> vierzehn schon nicht gewonnen haben, obwohl dort die Suche von selbst
> aufhörte.

Dazu fällt der Zuwachs: 6 → 8 brachte drei Zauber, 8 → 12 brachte drei bei
doppeltem Längenschritt.

### Die Verteilung ist der eigentliche Befund

Bei Länge 12, je Boss eindeutig gezählt:

| Zauber                     |     Bosse |
| -------------------------- | --------: |
| `searing`, `rot`, `kindle` | 17 von 17 |
| `smoulder`, `pyre`         |        16 |
| `bellows`, `cinder`        |        10 |
| `quench`                   |         6 |
| `riposte`, `virulence`     |         5 |
| neun weitere               |   1 bis 4 |

Das Issue heißt „vier Bosse, eine Zusammenstellung". Für den **Kern** stimmt
das — fünf Zauber stehen fast überall. Am Rand gibt es Variation, und sie war
beim Vergleich von vier Bossen _einer_ Instanz unsichtbar.

**Ein Teil dieser Variation war ein Artefakt der Suchlänge.** Die sieben
Zauber, die bei Länge 6 bei genau einem Boss standen, bei Länge 12:

```
cinder  1 → 10     riposte 1 → 5     efflux 1 → 4
aether  1 →  3     crucible-skin 1 → 3
plate   1 →  1     vapour  1 → 1
```

`cinder` war nie schwach — es hatte nur keinen Platz.

### Zwei fremde Zahlen, die dadurch einzuordnen sind

**Worker-3s Messung zu #509** (`docs/berichte/pruefstein-509.md`, „Die Zahl
misst zum Teil die Suchlänge") führt 48 tote Zauber bei Länge 6 und 41 bei
Länge 12. Seine Zahl war zusätzlich von der **Bosszahl** gedeckelt: Bei sechs
Zeilen über fünf Bosse passen höchstens dreißig Zauber hinein. Die Messung
hier hat 6 × 17 = 102 Plätze für 15 Zauber — sie stieß an keine Kombinatorik.
Die Aussage unter seiner Zahl hält; ihr Deckel ist jetzt bekannt.

**Der Kreuzvergleich fällt hier ein Urteil**, wo er bei fünf Bossen kippte
(0× `ACHTUNG`, 0× „KEIN Urteil" bei Länge 12). Zwei Erklärungen sind möglich —
der Nachlauf aus #833 oder die kleinere Streuung bei 17 statt 5 Bossen. **Nicht
gemessen**, welche zutrifft; der Zuwachs pro Boss (hier geringer als bei ihm)
spricht eher für die zweite.

## AC2 — warum die 37 nirgends vorkommen

**Ein Satz statt siebenunddreißig:**

> Ein toter Zauber kostet mehr Zeit, als seine Wirkung einbringt. **Wie viel,
> hängt vom Boss ab** — gemessen zwischen +3 und +97 Prozent, und an manchen
> Bossen ist es der Unterschied zwischen Sieg und Niederlage.

### Der Aufbau

Beste gefundene Liste, darin **ein lebender Zauber durch einen toten ersetzt**
— nicht eingefügt. Gleiche Listenlänge, gleiche GCD-Last, ein Zauber
Unterschied. Eingefügt statt ersetzt hätte zwei Größen gleichzeitig geändert.

```
cargo run --release --example balance -- --boss <id> --apl sim/apl/<datei>.apl
```

|                                       | Zeit bis Sieg | Siege                         |
| ------------------------------------- | ------------: | ----------------------------- |
| **boss-1-1, 12 Zeilen** (BASIS 25500) |         +92 % | `wisp` **und** `crust` 0/1000 |
| **boss-1-1, 6 Zeilen** (BASIS 30010)  |   +60 … +97 % | `wisp` 0/1000                 |
| **boss-3-2, 6 Zeilen** (BASIS 32000)  |    +3 … +23 % | alle 1000/1000                |

**Bei gleicher Listenlänge trennen die beiden Bosse um Faktor vier bis
zwanzig** — es ist der Boss, nicht die Suchlänge. Die Trennung wurde
ausdrücklich nachgemessen, weil beide Erklärungen plausibel waren.

`crust` ist das schärfste Beispiel: an `boss-1-1` null Siege, an `boss-3-2` der
höchste Schaden der ganzen Tabelle.

### Drei einfachere Antworten, die vorher gefallen sind

Sie stehen hier, damit sichtbar ist, warum AC2 so vorsichtig formuliert ist.

| Vermutung                                         | Widerlegung                                                                                                                        |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| „Einzelstücke sterben am fehlenden Zusammenspiel" | 28 % der Lebenden gegen 32 % der Toten tragen eine Wirkungsart, die sonst niemand hat. Sechs **lebende** Zauber sind Einzelstücke. |
| „Sie sind zu schwach"                             | Die Toten haben den **höheren** Grundschaden (Mittel 22 gegen 7, max 360 gegen 100). 19 der 21 Lebenden haben `baseDamage 0`.      |
| „Doppelt so lange, gewinnt nie"                   | Gilt für `boss-1-1`. An `boss-3-2` kostet derselbe Tausch 3 bis 23 Prozent, und alle gewinnen.                                     |

### Eine Spalte, die aus der Tabelle geflogen ist

„Schaden angekommen" (`sim/src/balance.rs:507`) ist
`damage_taken + absorbed` — **der Schaden am Spieler, nicht am Boss.** Sie
misst etwas anderes, als ihr Name nahelegt; eine Normierung rettet sie nicht.

Der Detektor war nicht die Quelle, sondern ein Widerspruch: `wisp` und `crust`
trugen mit 6720 den höchsten Wert der Tabelle und gewannen als einzige nie.
Für eine Schadensgröße wäre das verkehrt herum.

Damit fällt auch die daraus gerechnete „Rate": Sie war die **Schlagrate des
Bosses**, die nach fester Zeitlinie ohnehin konstant ist.

### Eine Beobachtung ohne Erklärung

`crust` fällt bei zwölf Zeilen aus und gewinnt bei sechs. Das belegt **nicht**
„mehr Platz schadet": Die beiden Basislisten sind nicht dieselbe Liste mit mehr
Platz, sondern zwei verschiedene Listen — die Umgebung des getauschten Platzes
ist in beiden Fällen eine andere. Nicht nachgemessen; AC2 hängt nicht daran.

## AC3 — die Ausgänge, und welcher je einen Kampf entschieden hat

Der Läufer führt fünf (`sim/src/balance.rs:216–227`):

| Ausgang        | Bedeutung                        | hat je entschieden?                                              |
| -------------- | -------------------------------- | ---------------------------------------------------------------- |
| `damage`       | Schaden am Boss                  | **ja** — die Siegschwelle liegt exakt bei den Boss-Lebenspunkten |
| `damage_taken` | Schaden am Spieler, nach WARD    | nein                                                             |
| `absorbed`     | von der Schutzschicht abgefangen | nein                                                             |
| `healed`       | zurückgeholtes Leben             | nein                                                             |
| `boss_healed`  | Heilung des Bosses               | nein                                                             |

```
Tode über ALLE Läufe dieser Messreihe:   0 von 1000, in jedem einzelnen
```

**Der Spieler stirbt nie.** Nicht bei der besten Liste, nicht bei der
schlechtesten, an keinem Boss, bei keiner Listenlänge. Jede Niederlage ist ein
Überdruck.

### Warum der Spieler nicht stirbt — nicht, weil er es nicht könnte

Der naheliegende Schluss wäre „kein Boss kann den Spieler töten". **Er ist
falsch**, und die Rechnung dagegen ist einfach:

```
boss-1-1   timelineLoopMs 15000, Summe je Zyklus 2100
           (900 bei 3000 ms, 300 bei 8000 ms, 900 bei 13000 ms)
Spieler    vessel 5000                        (data/player/base.json)
```

Über die vollen 60 Sekunden Überdruckzeit wären das vier Zyklen und 8400 roher
Schaden gegen 5000 Lebenspunkte. **Der Boss kann töten.**

Gemessen im Basislauf, nach WARD und Schutzschicht:

```
angekommen 2640, davon abgefangen 720   →  netto erlitten 1920
in 30010 ms                             →  64 Lebenspunkte je Sekunde
5000 Lebenspunkte reichen damit für     →  78151 ms
Überdruck von boss-1-1                  →  60000 ms
```

**Der Spieler würde nach rund 78 Sekunden sterben. Der Kampf endet nach 60.**

Der Abstand ist Faktor 1,3 — nicht zwei, nicht zehn. Der Spieler überlebt
nicht, weil ihm nichts gefährlich wird, sondern weil der Kampf vorher aufhört.

### Und er verteidigt sich dabei nicht

```
quench (einziger Zauber mit interrupt: true) in den Basislisten:  0 mal
Spieler geheilt im Basislauf:                                     0
```

Keine der drei besten Rotationen enthält den Unterbrecher, und geheilt wird
nicht. Der Spieler überlebt **ohne jede Verteidigungshandlung** — weil er
schnell genug ist, nicht weil er sich schützt.

### Damit steht AC2 als Folgerung

Ein Zauber, der heilt oder schützt, zahlt einen GCD für eine Wirkung auf einen
Ausgang, der in der zur Verfügung stehenden Zeit **nie** über Sieg oder
Niederlage entscheidet. Nicht weil er zu schwach ist — weil der Kampf endet,
bevor seine Größe zählt.

Das erklärt zugleich, warum 19 der 21 lebenden Zauber `baseDamage 0` haben und
trotzdem gewinnen: Sie wirken über DoT, Spender und Verstärkung auf denselben
einen Ausgang.

## AC4 — was eine zweite tragfähige Zusammenstellung bräuchte

**Nicht neue Zauber. Zeit, in der die anderen vier Ausgänge zählen.**

### Die Zahl, und sie ist kleiner als erwartet

```
Tod des Spielers bei aktueller Schadensrate    78151 ms
Überdruck-Grenze von boss-1-1                  60000 ms
                                               ────────
Abstand                                        Faktor 1,30
```

**Dreißig Prozent.** Ein Kampf, der dreißig Prozent länger dauert — oder eine
Schadensrate, die dreißig Prozent höher liegt — macht den Tod zur Möglichkeit,
und ab da kauft ein Schutz- oder Heilzauber etwas, das seinen GCD wert ist.

Die eigenen AC2-Zahlen zeigen, dass dieser Bereich real erreicht wird: Die
Varianten mit toten Zaubern brauchen 48000 bis 59000 ms statt 30000. **Sie
kämpfen bereits in der Zone, in der Verteidigung zählen würde** — sie kommen
nur nicht dort an, weil die Suche sie vorher verwirft.

### Ein Hebel, der bereits existiert und nicht benutzt wird

`quench` unterbricht, und bei `boss-1-1` sind 1800 von 2100 Schaden je Zyklus
unterbrechbar (86 %), bei `boss-2-2` 2200 von 2400 (92 %). Ein unterbrochener
Cast macht **null** Schaden, nicht weniger (`src/core/combat.ts:3510`).

Die Verteidigung ist also gebaut. Sie lohnt sich nur nicht, solange ein GCD
mehr Schaden mehr wert ist als ein verhinderter Schlag.

### Warum das billiger ist als neue Zauber

Die 37 toten Zauber bedienen bereits 24 verschiedene Wirkungsarten, davon 16
bei genau einem Zauber. Das Material liegt vor; was fehlt, ist Zeit, in der es
etwas kauft. Ein neuer Zauber auf denselben einen Ausgang wäre der
zweiundzwanzigste Weg, Schaden zu machen.

### Was diese Zahlen nicht sagen

Sie sind **kein Balance-Vorschlag**. Ob die Überdruckzeit steigt, die Bosse
härter schlagen oder die Lebenspunkte sinken — das ist eine
Design-Entscheidung. Hier steht die Schwelle, ab der die anderen vier Ausgänge
messbar werden, nicht der Weg dorthin.

## Was nicht geprüft ist

- **Zwei Bosse von siebzehn** in der AC2-Reihe. Die Spanne +3 bis +97 Prozent
  ist die zwischen ihnen, nicht die des ganzen Bestands.
- **Ein ersetzter Platz je Liste.** Ob ein toter Zauber an anderer Stelle trägt,
  ist nicht ausgeschlossen — `cinder` (1 → 10 von 17) zeigt, dass die Stelle
  zählt.
- **Die 37** bleibt Obergrenze (drei Bosse an `LengthLimit`).
- **Die 78151 ms gelten für einen Boss und eine Rotation.** Sie sind aus einem
  Basislauf hochgerechnet (1920 erlittener Schaden in 30010 ms), nicht durch
  einen Kampf über 78 Sekunden belegt. Ob die Rate über die volle Zeit konstant
  bleibt, ist offen — `escalation` in `boss-1-1.json` spricht eher dagegen.
- **Ob die Faktoren „zu schwer" oder „zu leicht" bedeuten** — dazu gehört die
  zweite Messgröße aus GAME-DESIGN 20.5 Punkt 4, Zeit bis zum Tod, und die
  greift erst, wenn der Tod möglich ist.
