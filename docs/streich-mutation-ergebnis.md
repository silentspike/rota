---
id: streich-mutation-wirkfelder
status: Stable
ssot: false
refs:
  docs: [GAME-DESIGN.md]
---

# Streich-Mutationen über die Wirkfelder der Zauberdaten

**Die Frage.** Ein Feld kann geparst und danach ignoriert werden. Dass sein Name in
`sim/src` vorkommt, belegt keine Wirkung — das war die Einschränkung meiner eigenen
Messung zu #513, und sie ist berechtigt: Am 2026-08-13 waren `setSetEffects` (0 Aufrufer),
`extendActiveDots` (nur der erste Brand) und `buildEfficiencyView` (gerechnet und in
derselben Zeile weggeworfen) alle typgeprüft, alle in Prüfungen gedeckt und alle im Spiel
wirkungslos.

**Das Verfahren.** Für jedes Wirkfeld: aus allen Zaubern streichen, die es tragen, dann
beide Suiten vollständig laufen lassen (`cargo test --no-fail-fast --all-targets` und
`vitest run`), Namen aller gefärbten Fälle aufzeichnen, Datensatz zurücksetzen.
Werkzeug: `tools/streich-mutation.py`.

## Ergebnis

**Alle 58 Wirkfelder sind abgesichert.** 57 durch mindestens eine Prüfung, die
spezifisch auf dieses Feld anspricht; eines (`spender.consumesAll`) durch die
Erzeugung statt durch eine Prüfung — begründet, siehe unten.

**Bezugspunkt.** Gemessen an `a32898a`; gegengeprüft gegen `d14e1f6` (main am
2026-08-13): dieselben 58 Felder, keines neu, keines weggefallen. Zwischen beiden
Ständen sind nur Prüfungen dazugekommen, und die können Deckung nur erhöhen. Wer die
Tabelle später wieder heranzieht, prüft zuerst diese Liste — ein neues Wirkfeld in den
Zauberdaten steht hier nicht drin und ist deshalb **nicht** als gedeckt ausgewiesen,
sondern gar nicht gemessen. `tools/streich-mutation.py` neu laufen lassen kostet
rund anderthalb Stunden.

## Warum „eine Prüfung färbt" die falsche Frage ist

Formal wurden alle 58 Läufe rot. Das sagt nichts: `tests/parity/freshness.test.ts`
färbt bei 56 der 58 Streichungen, `jede-groesse-ist-gemessen.test.ts` bei 41. Wer beim
ersten roten Fall abbricht, misst nur, dass er die Daten angefasst hat.

Die Unterscheidung wurde deshalb nicht gesetzt, sondern **gemessen**: Ein Fall, der bei
fast allen Streichungen färbt, unterscheidet nichts; einer, der bei vieren färbt, benennt
genau diese vier. Die Spalte „seltenster Färber" trägt das Urteil.

Der Abstand ist kein Grenzfall: Das unspezifischste Feld hat 41/58, das nächste 4/58.
Zwischen 5 und 40 liegt nichts.

## `spender.consumesAll` — kein Deckungsloch

Das einzige Feld ohne spezifischen Prüffall. Zwei Gründe, beide gemessen:

1. **Die Streichung fällt auf denselben Default.** `data.ts:1302` liest
   `raw['consumesAll'] !== false`, beide Träger (`pyre`, `pyroclasm`) stehen auf `true`.
   Ein fehlendes Feld verhält sich also identisch zum vorhandenen — es gibt keine
   Wirkung, die eine Prüfung färben könnte.
2. **Der andere Wert ist gar nicht zulässig.** Auf `false` gekippt bricht der Bau ab:

   ```
   error: failed to run custom build command for `rota-sim`
     calcination.json/spells[pyre]/spender: consumesAll=false ist nicht umgesetzt;
     Schritt 1 kennt nur Entlade-Zauber, die die Ressource vollstaendig verbrauchen
   ```

Das Feld ist damit durch `sim/build.rs` gesichert, und zwar schärfer als durch eine
Prüfung. Der Befund ist nicht neu: `tests/unit/pyroclasm-spender.test.ts` hält ihn seit
Issue #492 fest („`true -> false` NUR FRISCHE"). Dieser Lauf reproduziert ihn, er
entdeckt ihn nicht.

**Grenze des Verfahrens, die daraus folgt:** Bei einem Feld, dessen Weglassen denselben
Default ergibt, ist Streichen blind. Dort gehört der Wert gekippt. Von den 58 Feldern
trifft das nur auf dieses eine zu.

## Zwei Fehlmessungen, die vorher korrigiert wurden

**Ein fremder Boss im Arbeitsbaum.** Der erste Lauf meldete dreizehn Felder als gedeckt,
die Träger-Zusicherung eingeschlossen. Alle dreizehn falsch: `test-lethal-cast.json`
(uncommittet, aus #472) ließ `parity_boss_state` färben, weil der Simulator
`data/bosses/` vom Dateisystem liest und das Spiel seine Liste woanders hält. Jede
angelegte Prüfung war erfüllt — „mindestens eine Prüfung färbt", Exit-Code 101, die
Zusicherung „4 Zauber getroffen". Nur der **Name** des roten Falls verriet es.
Gegenmaßnahme: Nullprobe vor der ersten Mutation, Abbruch bei rotem Baum.

**Ein Timeout ohne Aufräumen.** Der Bash-Timeout griff nach zehn Minuten, das `finally`
lief bei SIGTERM nicht — `searing` stand danach ohne `dot.tickDamage` im Baum. Jeder
folgende Lauf hätte gegen kaputte Daten gemessen. Gegenmaßnahme: SIGTERM-Handler mit
`sys.exit`, Ergebnisse nach jedem Feld geschrieben.

## Träger-Zusicherung

`dot.tickDamage` gestrichen (4 Zauber) färbt `4240 @ 40510 unveraendert` und
`tests/unit/reference-run-options.test.ts` — beides Wirkungstests. Gemessen gegen grüne
Nullprobe; die erste Fassung dieser Zusicherung war aus dem Fremdgrund oben rot und
belegte nichts.

## Tabelle

Sortiert nach Spezifität: oben die Felder, deren Nachweis am wenigsten für sie allein
spricht. „Seltenster Färber" ist die Zahl der Felder, bei denen der spezifischste rote
Fall dieses Feldes ebenfalls färbt — je kleiner, desto gezielter der Nachweis.

| Feld                           | rote Fälle | seltenster Färber | Urteil               | spezifischster Fall                                                    |
| ------------------------------ | ---------: | ----------------: | -------------------- | ---------------------------------------------------------------------- |
| `spender.consumesAll`          |          2 |             41/58 | GESICHERT (build.rs) | die Messung nennt keine Größen, die es nicht mehr gibt                 |
| `extendShieldMs`               |          7 |              4/58 | gedeckt              | mit HARDEN steht sie dort noch                                         |
| `dotExtendTicks`               |          8 |              4/58 | gedeckt              | EMBER verlängert beim WIRKEN, nicht erst beim Aufruf der Methode       |
| `consumesDots`                 |          6 |              4/58 | gedeckt              | PYROCLASM verbraucht die Brände beim Wirken                            |
| `spender.bonusMultiplier`      |         78 |              3/58 | gedeckt              | bei 15 Hitze 360 Schaden                                               |
| `shield.durationMs`            |         49 |              3/58 | gedeckt              | DIE GEGENRICHTUNG: ein UNERREICHT-Eintrag für eine geschriebene Art is |
| `heal.tickIntervalMs`          |         10 |              3/58 | gedeckt              | tests/unit/poultice.test.ts                                            |
| `heal.fromShieldPercent`       |         11 |              3/58 | gedeckt              | die_pruefsumme_stimmt_in_jedem_takt_jedes_laufs                        |
| `dot.tickDamage`               |         98 |              3/58 | gedeckt              | er gibt Schaden und löscht den Brand                                   |
| `dot.resourceGainPerTick.heat` |         60 |              3/58 | gedeckt              | EFERT DIESELBE ZAHL WIE DIE KAMPF-AUSWAHL                              |
| `decayWard.permillePerStack`   |         11 |              3/58 | gedeckt              | der Schlag bei 12000 ms liegt gerade noch darin                        |
| `decayWard.durationMs`         |         11 |              3/58 | gedeckt              | der Schlag bei 12000 ms liegt gerade noch darin                        |
| `control.effect`               |         84 |              3/58 | gedeckt              | C3 — casts zählt den gescheiterten Zauber weiter als gewirkt           |
| `control.cost`                 |         84 |              3/58 | gedeckt              | C3 — casts zählt den gescheiterten Zauber weiter als gewirkt           |
| `control.amountPercent`        |         26 |              3/58 | gedeckt              | C3 — casts zählt den gescheiterten Zauber weiter als gewirkt           |
| `control.amountPerPoint`       |         27 |              3/58 | gedeckt              | der Vorrat ist danach leer                                             |
| `control.amountMs`             |         57 |              3/58 | gedeckt              | ein Ausgeber ohne genug Punkte scheitert auch mit dem Effekt           |
| `buff.kind`                    |         49 |              3/58 | gedeckt              | SLAG läuft ab und schreibt genau einen Eintrag der Art 7, mit seiner A |
| `buff.gcds`                    |         49 |              3/58 | gedeckt              | SLAG läuft ab und schreibt genau einen Eintrag der Art 7, mit seiner A |
| `synergy.windowGcds`           |         49 |              2/58 | gedeckt              | CANKER öffnet FESTER ein Fenster                                       |
| `synergy.partner`              |         49 |              2/58 | gedeckt              | CANKER öffnet FESTER ein Fenster                                       |
| `spender.bonusThreshold`       |         60 |              2/58 | gedeckt              | ergibt ein anderes Ergebnis als die fehlerfreie Runde                  |
| `shield.maxStacks`             |         76 |              2/58 | gedeckt              | CRUST holt den SIEG zurück, den es ohne ihn nicht gibt                 |
| `reflect.sharePercent`         |         13 |              2/58 | gedeckt              | endet mit der Runde — das Fenster läuft nicht in die nächste hinein    |
| `reflect.durationMs`           |         13 |              2/58 | gedeckt              | endet mit der Runde — das Fenster läuft nicht in die nächste hinein    |
| `perDecayStackPercent`         |         25 |              2/58 | gedeckt              | CANKER legt an, FESTER lebt davon — welche, nicht ob                   |
| `heal.perMissingPercent`       |         11 |              2/58 | gedeckt              | heilt nach jedem weiteren Treffer MEHR — das ist die Aussage des Raste |
| `extendDecayMs`                |          7 |              2/58 | gedeckt              | mit SEPSIS liegt er bei 18000 ms noch — dort trennt die Halbierung     |
| `decayCap.durationMs`          |         14 |              2/58 | gedeckt              | die KAPAZITAET traegt jeden Zuschlag, den ein Zauber geben darf        |
| `decayCap.bonus`               |         14 |              2/58 | gedeckt              | die KAPAZITAET traegt jeden Zuschlag, den ein Zauber geben darf        |
| `cooldownReductionLongestOnly` |          7 |              2/58 | gedeckt              | PLATE verliert 2000, die anderen nichts                                |
| `convert.perSecond`            |         13 |              2/58 | gedeckt              | 17 Sekunden ergeben 204 — der Satz gilt, nicht die eine Zahl           |
| `convert.from`                 |         13 |              2/58 | gedeckt              | 17 Sekunden ergeben 204 — der Satz gilt, nicht die eine Zahl           |
| `conversion.sharePercent`      |         21 |              2/58 | gedeckt              | aus 100 Schaden werden 25 Schutz                                       |
| `conversion.durationMs`        |         16 |              2/58 | gedeckt              | aus 100 Schaden werden 25 Schutz                                       |
| `spender.resource`             |        109 |              1/58 | gedeckt              | abgerundet, und der Rest verfällt (18.4)                               |
| `spender.damagePerPoint`       |         98 |              1/58 | gedeckt              | unterscheidet einen einzigen Schadenspunkt                             |
| `shield.amount`                |         78 |              1/58 | gedeckt              | payloads.ts enthält keine handvergebene Platzangabe                    |
| `resourceGain.heat`            |        139 |              1/58 | gedeckt              | KINDLE allein lässt die Grenze, wo sie ist                             |
| `refundsInterruptedGcd`        |         10 |              1/58 | gedeckt              | ac2_die_ruestung_erlaesst_den_rest_der_wartezeit                       |
| `interrupt`                    |         52 |              1/58 | gedeckt              | ac1_beide_seiten_schreiben_dieselben_arten                             |
| `heal.overMs`                  |         10 |              1/58 | gedeckt              | heilt IM MOMENT DES WURFES nichts — das ist der Unterschied zu SALVE   |
| `heal.amount`                  |         34 |              1/58 | gedeckt              | SALVE drängt zurück, wenn es etwas zu heilen gibt — und sonst nicht    |
| `freesNextGcd`                 |          7 |              1/58 | gedeckt              | nach SUBLIME löst KINDLE keine aus — der dritte Zauber geht sofort     |
| `doublesNextCast`              |          8 |              1/58 | gedeckt              | ac2_der_naechste_zauber_wirkt_zweimal                                  |
| `dot.tickIntervalMs`           |        152 |              1/58 | gedeckt              | C3: die Energieleiste steigt allein nach der Uhr                       |
| `dot.refreshMode`              |        161 |              1/58 | gedeckt              | VERLÄNGERN addiert auf die Restzeit — dieselbe Lage, anderes Ergebnis  |
| `dot.durationMs`               |        154 |              1/58 | gedeckt              | EMBER verlängert JEDEN von ihnen, je um seinen EIGENEN Takt            |
| `dot.decayPerTick`             |         22 |              1/58 | gedeckt              | der_aufbau_ist_in_beiden_laeufen_gleich                                |
| `decay.stacks`                 |        100 |              1/58 | gedeckt              | BLIGHT: seine blosse EXISTENZ bewegt nichts                            |
| `cooldownResetCharges`         |         25 |              1/58 | gedeckt              | REFLUX gibt eine — und fällt nach der ersten Wahl                      |
| `cooldownReductionMs`          |         48 |              1/58 | gedeckt              | ac1_beide_seiten_ergeben_denselben_gesamtschaden_und_dieselben_wuerfe  |
| `cooldownReductionBelowMs`     |          8 |              1/58 | gedeckt              | der_traeger_der_schwelle_bewegt_ueberhaupt_etwas                       |
| `controlGain`                  |        129 |              1/58 | gedeckt              | BIND gibt mehr Kontroll-Punkte                                         |
| `consumesDecayStacks`          |          6 |              1/58 | gedeckt              | danach liegt keiner mehr                                               |
| `clearsDoom`                   |         13 |              1/58 | gedeckt              | der_zaehler_laeuft_takt_fuer_takt_gleich                               |
| `buff.target`                  |         53 |              1/58 | gedeckt              | SLAG bewegt den Schadens-Faktor nicht                                  |
| `buff.factor`                  |         51 |              1/58 | gedeckt              | das ist die Eigenschaft, auf der der Kurzschluss ruht                  |
