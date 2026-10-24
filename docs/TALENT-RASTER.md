# Das Talent-Raster — fünfzehn Wege beschrieben, achtzehn gebaut

> **Zweck:** Dieses Dokument hält fest, **welche Größe** jeder Talentweg bewegt und **wo** sie
> steht. Ohne es entsteht bei jedem Bau dieselbe Frage neu, und sie wurde bereits dreimal
> verschieden beantwortet.

> ### ⚠ Der Stand ist gemessen, nicht entworfen
>
> Jede Zahl in den Spalten „TS" und „Rust" ist die Anzahl der Stellen, die eine Änderung anfassen
> muss — von Worker-1 an `main` gezählt, nicht geschätzt. **Vor jedem Bau neu messen:** Die Zahlen
> altern mit jedem Zauber, der die Größe zusätzlich liest.

## Was 12.5 wirklich sagt — und was es nicht sagt

`GAME-DESIGN.md:1349` ist die **einzige** Stelle, die Wege benennt:

> | Struktur je Schul-Baum | Gemeinsamer Stamm → drei Wege
> (bei CALCINATION etwa: schneller aufbauen / stärker entladen / länger halten) |

Das „etwa" ist kein Füllwort. **Nur die drei von CALCINATION stehen im Design** — alle übrigen
sind hier entschieden worden, nicht dort nachgelesen. Wer sie ändern will, ändert dieses Dokument,
nicht `GAME-DESIGN.md`.

> Hier stand „neun der zwölf". Die Zahl folgte der Überschrift, und beide waren gealtert: Die
> Tabelle unten führt fünfzehn Wege, gebaut sind achtzehn. **Die Aussage selbst bleibt richtig** —
> was aus dem Design stammt, sind drei; alles andere ist Auslegung. Nur ihr Bezug war falsch.

Und `GAME-DESIGN.md:176` nennt für COAGULATION ebenfalls „drei Wege", meint dort aber die drei
**Mechaniken** (shield, heal, conversion). Dieselbe Zeichenkette, zwei Bedeutungen — die Quelle
mehrerer Fehlgriffe.

## Die Wege — fünfzehn hier beschrieben, achtzehn gebaut

> ### ⚠ Diese Überschrift hieß „Die zwölf Wege" und stimmte dreifach nicht
>
> Gemessen am 2026-08-11 gegen **`git show origin/main:data/player/talents.json`**, nach dem Merge
> von #751 — nicht gegen den Arbeitsbaum:
>
> ```text
>   CALCINATION    trunk build release hold  (+ Schlüsseltalent)
>   FIXATION       trunk build cost effect
>   SUBLIMATION    trunk reduction charges threshold
>   COAGULATION    trunk thicker longer stacks
>   PUTREFACTION   trunk build effect duration
>   GLOBAL         trunk build release hold
> ```
>
> **Die SUBLIMATION-Zeile stand hier zwei Stunden lang falsch** — als
> `trunk build release hold`, mit dem Zusatz „Gemessen am 2026-08-11". Sie war gegen `main`
> gemessen, **während #751 noch offen war**, und das Datum ließ sie glaubwürdiger aussehen, als
> sie war. Worker-1 hat es beanstandet, der Orchestrator hat es nachgemessen.
>
> Ein Messdatum belegt den Zeitpunkt, nicht den Bezugspunkt. Wer gegen `main` misst, während ein
> PR auf genau diese Datei wartet, misst den Stand von gestern und schreibt das Datum von heute
> darüber.
>
> |                            |                                    |
> | -------------------------- | ---------------------------------- |
> | Überschrift sagte          | zwölf                              |
> | die Tabelle darunter führt | **fünfzehn** — fünf Schulen à drei |
> | gebaut sind                | **achtzehn** — sechs Bäume à drei  |
>
> **Die Überschrift stimmte nicht einmal mit ihrer eigenen Tabelle überein.** Die zwölf stammten
> aus der Zeit, als SUBLIMATION noch fehlte; die Tabelle wuchs, die Zahl darüber nicht.
>
> **Der GLOBAL-Baum fehlt hier weiterhin ganz** — nicht aus Versehen, sondern weil dieses Dokument
> die Wege je **Schule** beschreibt und GLOBAL keine ist. Er trägt trotzdem dieselben drei Wege,
> und wer „achtzehn" mit „fünfzehn beschriebenen" verwechselt, sucht drei Zeilen, die es hier nie
> gab.

| Schule           | Weg         | Größe                                    |    TS |  Rust | Prüfsumme     |
| ---------------- | ----------- | ---------------------------------------- | ----: | ----: | ------------- |
| **CALCINATION**  | build       | `resourceGainPerRankPerMille`            |     — |     — | gebaut (#723) |
|                  | release     | `damagePerRankPerMille`                  |     — |     — | gebaut        |
|                  | hold        | `dotDurationPerRankPerMille`             |     — |     — | gebaut        |
| **FIXATION**     | Aufbau      | `controlGain`                            | **0** | **0** | keine neue    |
|                  | Preis       | `control.cost`                           |     3 |     3 | keine neue    |
|                  | Wirkung     | `control.amount`                         |     5 |     5 | keine neue    |
| **PUTREFACTION** | Aufbau      | `decay.stacks` · `dot.decayPerTick`      |     2 |     2 | keine neue    |
|                  | Wirkung     | `perDecayStackPercent`                   | **0** | **0** | keine neue    |
|                  | Haltbarkeit | `extendDecayMs` · `decayWard.durationMs` |     2 |     2 | keine neue    |
| **COAGULATION**  | dicker      | `shield.amount`                          |     2 |     2 | keine neue    |
|                  | länger      | `shield.durationMs`                      |     1 |     2 | keine neue    |
|                  | mehr Stapel | `shield.maxStacks`                       |     1 |     1 | keine neue    |
| **SUBLIMATION**  | Abzug       | `cooldownReductionMs`                    |     ? |     ? | keine neue    |
|                  | Rücksetzer  | `cooldownResetCharges`                   |     ? |     ? | keine neue    |
|                  | Schwelle    | `cooldownReductionBelowMs`               |     ? |     ? | keine neue    |

> **Überholt — markiert am 2026-10-24 (#419), nicht neu entschieden.** Seit #824 (2026-08-13)
> tragen drei Zeilen dieser Tabelle eine andere Größe als hier beschrieben, und GLOBAL steht
> nicht darin:
>
> ```text
>   PUTREFACTION Aufbau        decayCapPerRankPerMille      die OBERGRENZE der Stapel
>   PUTREFACTION Haltbarkeit   decayWardPerRankPerMille     der SCHUTZ je Stapel
>   SUBLIMATION  Abzug         convertPerRankPerMille       EFFLUX' Umwandlung
>   GLOBAL       build         buildDamagePerRankPerMille   der Schaden der Aufbau-Zauber
> ```
>
> Gemessen mit dem Befehl im Kopfkommentar von `data/player/talents.json`. Die Spalten TS und
> Rust sind für diese vier nicht neu gezählt. Die Tabelle selbst bleibt, wie sie war: Die beiden
> PUTREFACTION-Zeilen stehen im Widerspruch zum Abschnitt „Zwei Gruppen wurden ausgeschlossen"
> unten, und welche Festlegung gilt, ist nicht entschieden.

> **SUBLIMATIONs drei Wege sind seit #751 gebaut**, und zwar mit je eigener Größe:
> `sublimation-reduction` → `cooldownReductionPerRankPerMille`, `-charges` →
> `cooldownResetChargesPerRank`, `-threshold` → `cooldownThresholdPerRankPerMille`.
>
> Die drei `?` in den Spalten TS und Rust bleiben stehen: Sie zählen die **Stellen**, die eine
> Änderung anfassen muss, und die habe ich nicht gemessen. Eine geschätzte Zahl wäre hier
> schlimmer als ein Fragezeichen — der Kasten oben verlangt ausdrücklich, sie vor jedem Bau neu
> zu erheben.

**Keiner der hier beschriebenen fünfzehn Wege braucht eine neue Prüfsummen-Zeile.** Gemessen: Alle
bewegen Größen, die bereits in beiden Prüfsummen stehen.

> **Für GLOBAL ist das nicht gemessen.** Der Baum steht nicht in der Tabelle, und seine drei Wege
> tragen heute alle `damagePerRankPerMille` — denselben Topf wie der Stamm. Solange das so ist,
> braucht auch er keine neue Zeile; sobald jemand ihm eigene Größen gibt, gehört die Frage neu
> gestellt.
>
> **Überholt — markiert am 2026-10-24 (#419):** Er hat sie inzwischen. Seit #812 und #824 trägt
> GLOBAL `buildDamagePerRankPerMille` (build), `spenderDamagePerRankPerMille` (release) und für
> hold `dotDurationPerRankPerMille`, `decayDurationPerRankPerMille` und
> `shieldDurationPerRankPerMille`. Die Frage nach einer Prüfsummen-Zeile ist damit gestellt und
> hier nicht beantwortet.

## Zwei Wege sind schon gebaut — nur der Schalter davor fehlt

Das ist der häufigste Einzelbefund dieser Arbeit und er ist dreimal aufgetreten (SCORIA, dann
FIXATION, dann PUTREFACTION):

```ts
// combat.ts — FIXATIONs AUFBAU, in `applyFixation`
this.controlValue += Math.round(spell.controlGain * faktor);

// combat.ts — PUTREFACTIONs WIRKUNG, in `applyDecayStackBonus`
(DECAY_STACK_PERCENT + proStapel) * this.stackBuffs.factor(this.nowMsForBuffs);
```

> **Hier standen Zeilennummern, und sie waren nach zwei Stunden falsch.**
> `combat.ts:1838` zeigte auf 1912, `:2637` auf 2743 — verschoben durch #723 und
> die Merges danach. Auch der zitierte Code stimmte nicht mehr wörtlich
> (`this.controlBuffs.factor(timeMs)` heißt inzwischen `faktor`).
>
> Deshalb stehen hier jetzt **Datei plus Symbol**: Die Zeile altert bei jedem
> Einschub darüber, der Funktionsname trägt.

Beide Faktoren stehen seit einem früheren Zauber (`GRIPLOCK` #489, `VIRULENCE` #540) auf beiden
Seiten und paritätisch. Ein Talent braucht dort **keine neue Wirkstelle** — nur einen Weg, den
Faktor aus einem Talent statt aus einem Zauber zu speisen.

**Entschieden zu PUTREFACTIONs Wirkung:** Der Faktor liegt auf der **Summe** aus Schul-Basis
(`DECAY_STACK_PERCENT`, 6 %) und Zauber-Anteil. Ein Talent darauf hebt beides. Das bleibt so — alle
acht Buff-Ziele wirken heute auf die fertige Größe, nicht auf einen ihrer Summanden. Ein Talent,
das nur `proStapel` hebt, wäre eine neue Wirkungsart mit einer Sonderregel, die kein anderer Faktor
kennt.

## Zwei Gruppen wurden ausgeschlossen — sie verdoppeln je einen Zauber

Bei PUTREFACTION standen fünf Gruppen zur Wahl. Zwei sind gestrichen:

| Gruppe     | Größe                        | Warum nicht                                                                                       |
| ---------- | ---------------------------- | ------------------------------------------------------------------------------------------------- |
| Obergrenze | `decayCap.bonus`             | **das ist PUTRID BLOOM** — ein Weg, der die Obergrenze hebt, ist derselbe Zauber ohne Tastendruck |
| Schutz     | `decayWard.permillePerStack` | **das ist DECAY**                                                                                 |

Beide wären technisch baubar (je 1–2 Stellen, keine neue Prüfsummen-Zeile). Der Einwand ist
fachlich: Ein Weg, der einen Zauber verdoppelt, ist so wenig eine Wahl wie einer, der eine Mechanik
verdoppelt.

> **Widerspricht dem Bestand — markiert am 2026-10-24 (#419), nicht entschieden.** PR #824 hat
> nach der Entscheidung des Auftraggebers vom 2026-08-13 („Jeder der vier bekommt eine Wirkung
> anderer BAUART", Kommentar in #419) genau diese beiden Größen gewählt: `putrefaction-build`
> trägt seither `decayCapPerRankPerMille` (die Obergrenze), `putrefaction-duration`
> `decayWardPerRankPerMille` (den Schutz je Stapel). Der Ausschluss oben und der Bestand
> schließen einander aus. Welche Festlegung gilt, entscheidet der Auftraggeber; bis dahin bleibt
> dieser Abschnitt stehen, wie er war.

## Die Kennungen sind inzwischen sprechend — bei vier von sechs Bäumen

**Hier stand „Alle sechs Bäume tragen `trunk/build/release/hold`", und das ist überholt.** Gemessen
gegen `git show origin/main:data/player/talents.json`:

```text
CALCINATION    trunk build release hold          deckt sich mit der Absicht
GLOBAL         trunk build release hold          desgleichen
FIXATION       trunk build cost effect           sprechend
SUBLIMATION    trunk reduction charges threshold sprechend
COAGULATION    trunk thicker longer stacks       sprechend
PUTREFACTION   trunk build effect duration       sprechend
```

Beim Einbau der Wirkungen (#751 und davor) sind die Kennungen mitgezogen worden. Der alte Befund —
„`release` heißt bei COAGULATION _länger_, bei FIXATION _Wirkung_" — ist damit erledigt: Kein Baum
außer CALCINATION und GLOBAL benutzt `release` oder `hold` überhaupt noch.

Sie bleiben Daten, kein öffentlicher Vertrag; ein Spielstand, der sie referenziert, existiert
weiterhin nicht.

## Drei Fallen, jede einmal bezahlt

1. **`fight.rs` schiebt in `apply_coagulation` ein Wakeup auf `shield.duration_ms`.** Wer die
   Dauer hebt und das Wakeup vergisst, bekommt eine Schicht, die im Simulator länger hält als im
   Spiel. Derselbe Fehler ist bei DECAY schon einmal passiert. (Stand 2026-08-10: zwei Stellen
   im selben Block, `shield_expires_at` und der `EventKind::Wakeup` direkt darunter.)
2. **`stabilise(shield.amount)` in `combat.ts` liest denselben Wert ein zweites Mal.** Ein
   Talent auf „dicker" muss beide Stellen treffen, sonst stabilisiert die Schicht nach dem alten
   Wert — beide Tests grün, beide Werte falsch.
3. **Die Kreuzprobe ist der eigentliche Wächter.** Jeder Weg muss die **fremden** Größen unberührt
   lassen. Bei #723 gemessen: Setzt man `calcination-release` auf 0, fällt nicht nur „erhöht den
   Schaden", sondern auch „bewegt jeder Weg eine ANDERE Größe als die beiden anderen". **Nur der
   zweite Fall fängt ein vertauschtes Ziel.**

## Was noch offen ist

- **SUBLIMATIONs drei Wege sind GEBAUT — und hier stand zweimal das Gegenteil.** Gemessen gegen
  `git show origin/main:data/player/talents.json`, nach dem Merge von #751:

  ```text
  sublimation-trunk       damagePerRankPerMille            20
  sublimation-reduction   cooldownReductionPerRankPerMille 100
  sublimation-charges     cooldownResetChargesPerRank        1
  sublimation-threshold   cooldownThresholdPerRankPerMille  60
  ```

  **Jeder Weg bewegt eine EIGENE Größe.** Genau das Gegenteil dessen, was hier stand: „alle vier
  bewegen `damagePerRankPerMille` — denselben Topf". Diese Aussage war gegen `main` gemessen,
  **während #751 noch offen war**, und trug trotzdem ein Messdatum.

  Damit ist auch die Kreuzprobe-Regel (Falle 3 oben) für SUBLIMATION erfüllbar: Es gibt drei
  verschiedene Größen, die einander unberührt lassen müssen. Solange alle vier auf denselben Topf
  zeigten, war sie dort gar nicht baubar.

  GLOBAL bewegt weiterhin dreimal `damagePerRankPerMille` (15 je Rang) — dort trifft die alte
  Aussage zu, und dort allein.

  > **Überholt — markiert am 2026-10-24 (#419):** Beide Absätze darüber sind gealtert.
  > `sublimation-reduction` trägt seit #824 `convertPerRankPerMille` (120 je Rang, EFFLUX'
  > Umwandlung) statt `cooldownReductionPerRankPerMille`, und GLOBAL trägt seit #812 und #824
  > eigene Größen, siehe den Hinweis bei der Tabelle oben.

  Sechs der zwölf Zauber tragen die Zahlen, an denen die Wege greifen:

  ```
  cooldownReductionMs        vapour 1000 · wisp 500 · decant 2000 · ascend 10000
  cooldownResetCharges       reflux 1 · alembic 2
  cooldownReductionBelowMs   ascend 10000
  ```

  > **Wie der Irrtum entstand — und warum er hier steht.** Gemessen wurden die **letzten zwei** der
  > Liste (`sublime`, `condense`); beide tragen Schalter statt Zahlen, und daraus wurde auf alle
  > geschlossen. **Zwei von elf sind keine Messung über elf.** Die Aussage stand danach in diesem
  > Dokument, wurde dem Auftraggeber vorgelegt und trug eine Entscheidung — RETORTs Wiederaufnahme.
  > Ein Irrtum in einer Arbeitsgrundlage wandert weiter als einer im Code.

  RETORT (`#733`) ist trotzdem richtig gebaut worden: Er ist der erste Zauber, der die **Wartezeit
  selbst** anfasst, und damit eine vierte Größe — nicht nur ein zwölfter Zauber.

- **Die Schlüsseltalente** je Schule. Nur CALCINATION hat eines (`calcination-keystone`).
  `keystone: string` und `hasKeystone()` sind wiederverwendbar; jedes Schlüsseltalent ist ein
  eigenes Verhalten, kein Zahlendreher.

  > **Überholt — markiert am 2026-10-24 (#419):** Es sind fünf, je Schul-Baum eines
  > (`split-spender`, `partial-control`, `auto-reset`, `shield-refund`, `decay-echo`); der globale
  > Baum hat keines. Gemessen mit
  > `jq -r '[.trees[].talents[] | select(.keystone != null)] | length' data/player/talents.json`.
  > Wie viele es werden sollen — je Baum oder je Weg —, ist offen.

- **Die Verteilung der 50 Talentpunkte** über die Stufen (`talentPointTarget` in
  `data/player/levels.json`). Die Zahl
  ist entschieden, ihre Verteilung nicht. Warnung für den Wächter: Wer die Verteilung summiert und
  mit der Summe **derselben** Verteilung vergleicht, prüft seine eigene Erwartung gegen sich selbst.
