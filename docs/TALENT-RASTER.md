# Das Talent-Raster — zwölf Wege, neun entschieden

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

Das „etwa" ist kein Füllwort. **Neun der zwölf Wege stehen nirgends im Design** — sie sind hier
entschieden worden, nicht dort nachgelesen. Wer sie ändern will, ändert dieses Dokument, nicht
`GAME-DESIGN.md`.

Und `GAME-DESIGN.md:176` nennt für COAGULATION ebenfalls „drei Wege", meint dort aber die drei
**Mechaniken** (shield, heal, conversion). Dieselbe Zeichenkette, zwei Bedeutungen — die Quelle
mehrerer Fehlgriffe.

## Die zwölf Wege

| Schule | Weg | Größe | TS | Rust | Prüfsumme |
| ------ | --- | ----- | -: | ---: | --------- |
| **CALCINATION** | build | `resourceGainPerRankPerMille` | — | — | gebaut (#723) |
| | release | `damagePerRankPerMille` | — | — | gebaut |
| | hold | `dotDurationPerRankPerMille` | — | — | gebaut |
| **FIXATION** | Aufbau | `controlGain` | **0** | **0** | keine neue |
| | Preis | `control.cost` | 3 | 3 | keine neue |
| | Wirkung | `control.amount` | 5 | 5 | keine neue |
| **PUTREFACTION** | Aufbau | `decay.stacks` · `dot.decayPerTick` | 2 | 2 | keine neue |
| | Wirkung | `perDecayStackPercent` | **0** | **0** | keine neue |
| | Haltbarkeit | `extendDecayMs` · `decayWard.durationMs` | 2 | 2 | keine neue |
| **COAGULATION** | dicker | `shield.amount` | 2 | 2 | keine neue |
| | länger | `shield.durationMs` | 1 | 2 | keine neue |
| | mehr Stapel | `shield.maxStacks` | 1 | 1 | keine neue |
| **SUBLIMATION** | — | wartet auf RETORT | | | |

**Keiner der zwölf Wege braucht eine neue Prüfsummen-Zeile.** Gemessen: Alle bewegen Größen, die
bereits in beiden Prüfsummen stehen.

## Zwei Wege sind schon gebaut — nur der Schalter davor fehlt

Das ist der häufigste Einzelbefund dieser Arbeit und er ist dreimal aufgetreten (SCORIA, dann
FIXATION, dann PUTREFACTION):

```ts
// combat.ts:1838 — FIXATIONs AUFBAU
this.controlValue += Math.round(spell.controlGain * this.controlBuffs.factor(timeMs));

// combat.ts:2637 — PUTREFACTIONs WIRKUNG
const jeStapel = Math.round(
  (DECAY_STACK_PERCENT + proStapel) * this.stackBuffs.factor(...)
);
```

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

| Gruppe | Größe | Warum nicht |
| ------ | ----- | ----------- |
| Obergrenze | `decayCap.bonus` | **das ist PUTRID BLOOM** — ein Weg, der die Obergrenze hebt, ist derselbe Zauber ohne Tastendruck |
| Schutz | `decayWard.permillePerStack` | **das ist DECAY** |

Beide wären technisch baubar (je 1–2 Stellen, keine neue Prüfsummen-Zeile). Der Einwand ist
fachlich: Ein Weg, der einen Zauber verdoppelt, ist so wenig eine Wahl wie einer, der eine Mechanik
verdoppelt.

## Die Kennungen in `talents.json` lügen bei drei Wegen

Alle sechs Bäume tragen `trunk/build/release/hold`. Bei CALCINATION deckt sich das mit der Absicht,
bei den anderen nicht:

```
FIXATION      build → PREIS         release → WIRKUNG
PUTREFACTION  build → AUFBAU        release → HALTBARKEIT
COAGULATION   build → dicker        release → länger       hold → mehr Stapel
```

Bei COAGULATION passt **keine einzige**. „Mehr Stapel" ist kein „halten". Die Kennungen werden beim
Einbau der Wirkungen mitgezogen — sie sind Daten, kein öffentlicher Vertrag, und ein Spielstand,
der sie referenziert, existiert noch nicht.

## Drei Fallen, jede einmal bezahlt

1. **`fight.rs:1887` schiebt ein Wakeup auf `shield.duration_ms`.** Wer die Dauer hebt und das
   Wakeup vergisst, bekommt eine Schicht, die im Simulator länger hält als im Spiel. Derselbe
   Fehler ist bei DECAY schon einmal passiert.
2. **`stabilise(shield.amount)` (`combat.ts:1952`) liest denselben Wert ein zweites Mal.** Ein
   Talent auf „dicker" muss beide Stellen treffen, sonst stabilisiert die Schicht nach dem alten
   Wert — beide Tests grün, beide Werte falsch.
3. **Die Kreuzprobe ist der eigentliche Wächter.** Jeder Weg muss die **fremden** Größen unberührt
   lassen. Bei #723 gemessen: Setzt man `calcination-release` auf 0, fällt nicht nur „erhöht den
   Schaden", sondern auch „bewegt jeder Weg eine ANDERE Größe als die beiden anderen". **Nur der
   zweite Fall fängt ein vertauschtes Ziel.**

## Was noch offen ist

- **SUBLIMATIONs drei Wege.** Die Schule steht bei elf Zaubern, ihr Größen-Vorrat aber unverändert
  bei vier — und drei davon liegen in je **einem** Zauber. Ein Weg träfe dort einen Zauber statt
  einen Stil. RETORT soll eine fünfte Größe bringen, nicht einen zwölften Zauber.
- **Die Schlüsseltalente** je Schule. Nur CALCINATION hat eines (`calcination-keystone`).
  `keystone: string` und `hasKeystone()` sind wiederverwendbar; jedes Schlüsseltalent ist ein
  eigenes Verhalten, kein Zahlendreher.
- **Die Verteilung der 50 Talentpunkte** über die Stufen (`data/player/levels.json:26`). Die Zahl
  ist entschieden, ihre Verteilung nicht. Warnung für den Wächter: Wer die Verteilung summiert und
  mit der Summe **derselben** Verteilung vergleicht, prüft seine eigene Erwartung gegen sich selbst.
