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

## Die Einteilung der 34 Bau-Felder (Refs #1076)

Erstellt am 2026-08-22 ohne Lauf, gelesen aus `sim/build.rs`, `sim/src/data.rs` und
`sim/src/fight.rs`.

> **`BAU` heißt „keine gemessene Deckung", nicht „ungedeckt".** Ein Feld, dessen Streichung den
> Bau bricht, kann sehr wohl geprüft sein — die Streich-Methode kann es nur nicht zeigen, weil
> nach dem Baubruch keine Prüfung mehr läuft. Wer diese Einteilung liest, schreibt deshalb nicht
> ungeprüft Tests für etwas, das längst geprüft ist: Die Gruppen sagen, **welche Absicherung
> sinnvoll wäre**, nicht dass keine existiert.

### Gruppe A — Bauzeit-Zusicherungen: 4 Felder, kein Prüffall möglich

Diese vier werden von `build.rs` gelesen, geprüft und **verworfen**. Sie landen in keiner
Rust-Struktur; der Simulator sieht sie nie.

| Feld                  | Zusicherung in `build.rs`        | zulässige Werte |
| --------------------- | -------------------------------- | --------------- |
| `convert.from`        | `assert_eq!(from, "cooldown")`   | genau einer     |
| `spender.resource`    | `assert_eq!(named, resource_id)` | genau einer     |
| `spender.consumesAll` | `assert!(consumes_all)`          | nur `true`      |
| `dot.refreshMode`     | `assert_eq!(mode, "reset")`      | nur `"reset"`   |

**Ein Prüffall wäre hier wertlos.** Der Wert lässt sich nicht variieren, ohne dass der Bau
bricht — die `BAU`-Meldung _ist_ die Absicherung und nicht ihr Fehlen.

`spender.resource` ist der wertvollste der vier: Er prüft, dass ein Zauber dieselbe Ressource
verbraucht, die seine Schule führt — eine Kreuzprobe zwischen zwei Datendateien, die sonst
niemand zieht.

**Das ist nicht die Klasse der Faktoren-Töpfe aus #420.** Dort lag eine Zahl, die niemand las;
hier liegt eine Zusicherung, die genau einmal wirkt und dann nicht mehr gebraucht wird.

### Gruppe B — Zahlen in einer Rechnung: 28 Felder, Prüffall billig und wirksam

Alle fließen in eine Rust-Struktur **und** werden in `fight.rs` gelesen. Je Feld eine
Fundstelle:

| Feld                                                                                                    | gelesen in                                           |
| ------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `spender.damagePerPoint`                                                                                | `fight.rs:4709`                                      |
| `spender.bonusThreshold`                                                                                | `fight.rs:4710`                                      |
| `spender.bonusMultiplier`                                                                               | `fight.rs:4711`                                      |
| `dot.tickDamage`                                                                                        | `fight.rs:3273`                                      |
| `dot.tickIntervalMs`, `dot.durationMs`                                                                  | über `Dot`, mit `tick_damage` zusammen               |
| `shield.amount`, `shield.durationMs`                                                                    | `fight.rs`, 20 Stellen über `self.shield`            |
| `shield.maxStacks`                                                                                      | `fight.rs:2418`                                      |
| `heal.amount`, `heal.tickIntervalMs`                                                                    | `fight.rs:2483`                                      |
| `reflect.sharePercent`, `reflect.durationMs`                                                            | `self.reflect_percent`, `self.reflect_until`         |
| `conversion.sharePercent`, `conversion.durationMs`                                                      | `self.conversion_percent`                            |
| `decayCap.bonus`, `decayCap.durationMs`                                                                 | `self.decay_cap_bonus`                               |
| `decayWard.permillePerStack`, `decayWard.durationMs`                                                    | `self.decay_ward_permille`                           |
| `convert.perSecond`                                                                                     | `fight.rs`, in der Schadensrechnung                  |
| `control.cost`, `control.effect`, `control.amountMs`, `control.amountPercent`, `control.amountPerPoint` | `fight.rs`, 12 Stellen                               |
| `buff.kind`, `buff.factor`, `buff.gcds`                                                                 | `fight.rs:4795` `apply_for_gcds(kind, factor, gcds)` |
| `resourceGain.heat`, `dot.resourceGainPerTick.heat`                                                     | `fight.rs`, Ressourcenaufbau                         |

**Für diese ist ein Prüffall billig:** Der Wert steht in einer Rechnung, eine Änderung verschiebt
eine Zahl im Kampf.

> **`control.amountMs`, `.amountPercent` und `.amountPerPoint` sind kein Dreierpaket, sondern ein
> Feld.** `build.rs` wählt nach `control.effect` aus, welches gelesen wird, und schreibt den Wert
> in dasselbe Rust-Feld `amount`:
>
> | `effect`                                                   | gelesenes Feld   |
> | ---------------------------------------------------------- | ---------------- |
> | `slowCast`, `delayStrike`, `allThree`, `makeInterruptible` | `amountMs`       |
> | `holdPhase`                                                | `amountPercent`  |
> | `convertToDamage`                                          | `amountPerPoint` |
>
> Ein Zauber trägt nur eines davon. Die Tabelle zählt sie dreifach, obwohl **ein** Prüffall alle
> drei deckt.
>
> Dazu eine Stelle, die beim Bauen eines Prüffalls zu kennen ist: Für `PIN` bedeutet `amountMs`
> etwas anderes als bei `delayStrike` — dort ist es die **Fensterlänge**, mit der ein Schlag ohne
> Wirkzeit unterbrechbar wird, keine Verzögerung. Der Kommentar in `build.rs` sagt es ausdrücklich
> („Dasselbe Feld, andere Bedeutung"). Ein Prüffall auf `amountMs` deckt also zwei Bedeutungen.

### Gruppe C — Kennungen und Namen: keine

Unter den 34 ist kein reines Namensfeld. Die vier Textfelder (`convert.from`, `spender.resource`,
`dot.refreshMode`, `control.effect`) sind entweder Zusicherungen (Gruppe A) oder
Auswahlschalter, die eine Rechnung steuern (`control.effect`, Gruppe B).

### AC4 — Felder, die von keinem Pfad gelesen werden: keine

Geprüft wurde in zwei Schritten:

1. **Struktur:** Für jedes der zwölf betroffenen `data.rs`-Structs (`Convert`, `Dot`, `Buff`,
   `DecayWard`, `DecayCap`, `Spender`, `Shield`, `Heal`, `Conversion`, `Reflect`, `Control`,
   `ResourceGain`) die Feldliste gelesen. Vier JSON-Felder haben dort keine Entsprechung — das
   sind genau die aus Gruppe A.
2. **Leser:** Für jede der zwölf Strukturen gemessen, ob sie in `fight.rs` gelesen wird. Alle
   zwölf werden gelesen; für die selteneren (`reflect`, `conversion`, `decayCap`, `decayWard`,
   `convert`) zusätzlich im Kontext nachgesehen, dass **beide** ihrer Felder verwendet werden.

**Die Liste ist leer, und das ist ein Ergebnis, kein Ausbleiben.** Kein Feld unter den 34 ist
gebaut und ungelesen. Die vier aus Gruppe A werden gelesen — nur eben zur Bauzeit und nicht im
Kampf.

### Was hier nicht gemessen ist

Ob die 28 Felder der Gruppe B auf der **TypeScript-Seite** ebenfalls gelesen werden. Gemessen
wurden `fight.rs`, `balance.rs` und `gear.rs`, nicht `src/core/`. Für die Frage „ist ein Prüffall
billig" reicht die Rust-Seite; für die Parität nicht.
