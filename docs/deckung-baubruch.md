# Deckungstabelle — welche Felder sind wirklich geprüft?

Erzeugt am 2026-08-22 aus `tools/streich-mutation.py --nur-bau`, 60 Felder in 22 Minuten (Refs #1062).

## Was hier gemessen wurde — und was nicht

Die Spalte **Bau** ist gemessen: Jedes Feld wurde aus den Zauberdaten gestrichen und `cargo remote -c -- test --no-run --all-targets` gefahren.

Die Spalten **alt** stammen aus dem vorigen Durchgang und sind **übernommen, nicht neu gemessen**.

| Wert   | Bedeutung                                                                                                                                                                                                         |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `BAU`  | Die Streichung bricht den Bau. **Es lief keine einzige Prüfung.** Für dieses Feld gibt es _keine gemessene Deckung_ — das heißt nicht, dass es ungetestet ist, sondern dass diese Methode dazu nichts sagen kann. |
| `baut` | Der Bau geht durch. Das alte Urteil beruht also auf einem Lauf, der stattgefunden hat.                                                                                                                            |

**Die Zahl in _alte Namen_ zählt Namen, nicht gefärbte Fälle** — die Auswertung dedupliziert, ein Testname aus 38 Dateien erscheint als einer. Die Zahl ist eine Untergrenze.

## Die Tabelle

| Feld                           | Zauber | Bau (gemessen) | altes Urteil (übernommen)   | alte Namen (übernommen) |
| ------------------------------ | -----: | -------------- | --------------------------- | ----------------------: |
| `channelMs`                    |      1 | baut           | — (neu, kein alter Eintrag) |                       — |
| `channelTickMs`                |      1 | baut           | — (neu, kein alter Eintrag) |                       — |
| `clearsDoom`                   |      1 | baut           | gedeckt                     |                      13 |
| `consumesDecayStacks`          |      1 | baut           | gedeckt                     |                       6 |
| `consumesDots`                 |      1 | baut           | gedeckt                     |                       6 |
| `control.amountPerPoint`       |      1 | **BAU**        | gedeckt                     |                      27 |
| `convert.from`                 |      1 | **BAU**        | gedeckt                     |                      13 |
| `convert.perSecond`            |      1 | **BAU**        | gedeckt                     |                      13 |
| `cooldownReductionBelowMs`     |      1 | baut           | gedeckt                     |                       8 |
| `cooldownReductionLongestOnly` |      1 | baut           | gedeckt                     |                       7 |
| `decayCap.bonus`               |      1 | **BAU**        | gedeckt                     |                      14 |
| `decayCap.durationMs`          |      1 | **BAU**        | gedeckt                     |                      14 |
| `decayWard.durationMs`         |      1 | **BAU**        | gedeckt                     |                      11 |
| `decayWard.permillePerStack`   |      1 | **BAU**        | gedeckt                     |                      11 |
| `dot.decayPerTick`             |      1 | baut           | gedeckt                     |                      22 |
| `dotExtendTicks`               |      1 | baut           | gedeckt                     |                       8 |
| `doublesNextCast`              |      1 | baut           | gedeckt                     |                       8 |
| `extendDecayMs`                |      1 | baut           | gedeckt                     |                       7 |
| `extendShieldMs`               |      1 | baut           | gedeckt                     |                       7 |
| `freesNextGcd`                 |      1 | baut           | gedeckt                     |                       7 |
| `heal.fromShieldPercent`       |      1 | baut           | gedeckt                     |                      11 |
| `heal.overMs`                  |      1 | baut           | gedeckt                     |                      10 |
| `heal.perMissingPercent`       |      1 | baut           | gedeckt                     |                      11 |
| `heal.tickIntervalMs`          |      1 | **BAU**        | gedeckt                     |                      10 |
| `interrupt`                    |      1 | baut           | gedeckt                     |                      52 |
| `reflect.durationMs`           |      1 | **BAU**        | gedeckt                     |                      13 |
| `reflect.sharePercent`         |      1 | **BAU**        | gedeckt                     |                      13 |
| `refundsInterruptedGcd`        |      1 | baut           | gedeckt                     |                      10 |
| `synergy.partner`              |      1 | baut           | gedeckt                     |                      49 |
| `synergy.windowGcds`           |      1 | baut           | gedeckt                     |                      49 |
| `control.amountPercent`        |      2 | **BAU**        | gedeckt                     |                      26 |
| `conversion.durationMs`        |      2 | **BAU**        | gedeckt                     |                      16 |
| `conversion.sharePercent`      |      2 | **BAU**        | gedeckt                     |                      21 |
| `cooldownResetCharges`         |      2 | baut           | gedeckt                     |                      25 |
| `decay.stacks`                 |      2 | baut           | gedeckt                     |                     100 |
| `spender.bonusMultiplier`      |      2 | **BAU**        | gedeckt                     |                      78 |
| `spender.bonusThreshold`       |      2 | **BAU**        | gedeckt                     |                      60 |
| `spender.consumesAll`          |      2 | **BAU**        | GESICHERT (build.rs)        |                       2 |
| `spender.damagePerPoint`       |      2 | **BAU**        | gedeckt                     |                      98 |
| `spender.resource`             |      2 | **BAU**        | gedeckt                     |                     109 |
| `controlGain`                  |      3 | baut           | gedeckt                     |                     129 |
| `dot.resourceGainPerTick.heat` |      3 | **BAU**        | gedeckt                     |                      60 |
| `perDecayStackPercent`         |      3 | baut           | gedeckt                     |                      25 |
| `resourceGain.heat`            |      3 | **BAU**        | gedeckt                     |                     139 |
| `cooldownReductionMs`          |      4 | baut           | gedeckt                     |                      48 |
| `dot.durationMs`               |      4 | **BAU**        | gedeckt                     |                     154 |
| `dot.refreshMode`              |      4 | **BAU**        | gedeckt                     |                     161 |
| `dot.tickDamage`               |      4 | **BAU**        | gedeckt                     |                      98 |
| `dot.tickIntervalMs`           |      4 | **BAU**        | gedeckt                     |                     152 |
| `shield.amount`                |      4 | **BAU**        | gedeckt                     |                      78 |
| `shield.durationMs`            |      4 | **BAU**        | gedeckt                     |                      49 |
| `shield.maxStacks`             |      4 | **BAU**        | gedeckt                     |                      76 |
| `control.amountMs`             |      5 | **BAU**        | gedeckt                     |                      57 |
| `heal.amount`                  |      5 | **BAU**        | gedeckt                     |                      34 |
| `buff.factor`                  |      7 | **BAU**        | gedeckt                     |                      51 |
| `buff.gcds`                    |      7 | **BAU**        | gedeckt                     |                      49 |
| `buff.kind`                    |      7 | **BAU**        | gedeckt                     |                      49 |
| `buff.target`                  |      7 | baut           | gedeckt                     |                      53 |
| `control.cost`                 |      7 | **BAU**        | gedeckt                     |                      84 |
| `control.effect`               |      7 | **BAU**        | gedeckt                     |                      84 |

**60 Felder gefahren.** 34 mal BAU, 26 mal baut.

## Der Vergleich mit der bisherigen Tabelle

|                                                     |        |
| --------------------------------------------------- | -----: |
| Felder, die als `gedeckt` standen und **BAU** sind  | **33** |
| alt schon als `GESICHERT (build.rs)` erkannt        |      1 |
| bestätigt: alt `gedeckt`, Bau geht durch            |     24 |
| ohne alten Eintrag (neu seit dem letzten Durchgang) |      2 |

57 Felder standen als `gedeckt`. **33 davon haben keine gemessene
Deckung** — 58 Prozent. Alle trugen zweistellige Namenszahlen; `control.amountPerPoint`
sogar 27. Wer die Tabelle las, sah dort die beste Deckung der Liste.

Die Zahlen kamen aus der TypeScript-Suite, während der Rust-Bau gebrochen am Boden lag.
Der Mechanismus ist in #1045 beschrieben: `rc != 0` führte zu ROT, und ROT hieß gedeckt.

## Ein Muster, das ich nicht erklären kann

Die 34 treten in Gruppen auf, nicht einzeln:

    control.*    amountMs, amountPercent, amountPerPoint, cost, effect
    dot.*        durationMs, refreshMode, tickDamage, tickIntervalMs,
                 resourceGainPerTick.heat
    spender.*    bonusMultiplier, bonusThreshold, consumesAll,
                 damagePerPoint, resource
    shield.*     amount, durationMs, maxStacks

Es sind **ganze Strukturen**, nicht verstreute Felder. Das legt nahe, dass `build.rs` den
vollständigen Block verlangt und jede Lücke darin den Bau zerstört. **Gemessen ist das
nicht** — es ist eine Lesart der Liste.

## Was diese Tabelle nicht beantwortet

Drei Fälle liefern `baut` und sind trotzdem keine Deckungsaussage:

1. **GRÜN gegen ROT.** `baut` heißt nur, dass ein Lauf stattfand — nicht, dass er etwas
   bemerkt hat. Diese Unterscheidung steht in der Spalte _altes Urteil_ und ist übernommen.

2. **HÄNGT.** `channelTickMs` baut sauber durch (40 s) und steht deshalb als `baut`.
   Sein _Lauf_ endet nie: Am 2026-08-22 verbrauchten vier vitest-Worker über
   770 CPU-Sekunden ohne Ergebnis. Im Lauf-Modus hätte das nach 30 Minuten eine
   ungefangene `TimeoutExpired` geworfen und den ganzen Durchgang getötet — das Feld
   stünde dann in **keiner** Zeile, und eine fehlende Zeile sieht aus wie ein Feld,
   das es nicht gibt.

3. **OOM.** Ein an Speichermangel gestorbener Lauf liefert `rc != 0` und sähe im vollen
   Weg wie ROT aus. In diesem Durchgang fällt er nicht an, weil keine Suite läuft.

## Fremdlast während der Messung

Protokolliert mit `tools/last-protokoll.sh`, Takt 30 s, 166 Zeilen von 00:56:53 bis 02:19:59.

|                                       |                                               |
| ------------------------------------- | --------------------------------------------: |
| Messpunkte im Durchgang (01:31–01:54) |                                            51 |
| davon mit Fremdlast                   |                                    51 (100 %) |
| Spitze                                | 100 % aus `/work`, Firefox durchgehend 37,9 % |

**Die Zahl ist eine Untergrenze.** Ein `proxmox-backup-client` lief ab 01:19:13 mit 85 %
und steht in keiner Zeile: `readlink /proc/<pid>/cwd` schlägt bei Prozessen anderer
Benutzer fehl, und das Werkzeug übersprang sie stillschweigend. Behoben — unlesbare
Herkunft wird jetzt als `?(benutzer)` gemeldet statt verschwiegen.

Auf die Klassifikation wirkt sich das nicht aus: Gemessen wurde ein **Bau** auf rustbuild,
und ein Bau hat keine Zeitschranken. Fremdlast auf dieser Maschine kann ihn verlangsamen,
nicht sein Ergebnis ändern.
