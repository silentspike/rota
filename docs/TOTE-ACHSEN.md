# Tote Achsen — Bestandsaufnahme (Issue #480, 2026-08-11)

Erhoben auf Bitte des Auftraggebers, nachdem in zwei Tagen **vier** Größen
gefunden wurden, die gebaut, vorhanden und nie gelesen waren: `upgradeSteps`
(#740), `gearEffects` (#748), `activeRotationEffects` (#774) und
`GlobalCooldown.setLength` (#777 — seit #177, also Wochen).

## Wie gemessen wurde, und der Beleg dass es misst

Gesucht wird das **Symbol im Dateitext**, nicht ein Aufrufmuster: Ein
mehrzeilig umbrochener Aufruf ist für ein Zeilenmuster unsichtbar.

Gezählt je Symbol: Treffer in `src/` bzw. `sim/src/` (produktiv) gegen Treffer
in `tests/` bzw. `sim/tests/`. Ein Symbol mit höchstens einem produktiven
Treffer wird nur an seiner Definition genannt.

**Träger-Zusicherung, in beide Richtungen belegt** — ein Werkzeug, das nicht
anschlägt, misst nichts:

| Probe          | Stand    | produktiv | Tests | Urteil                                       |
| -------------- | -------- | --------: | ----: | -------------------------------------------- |
| `setLength`    | vor #777 |         1 |     4 | **wird gemeldet** — richtig, es war tot      |
| `setLength`    | heute    |         3 |     5 | nicht gemeldet — richtig, #777 hat es belebt |
| `wornFrom`     | heute    |        22 |    85 | nicht gemeldet — richtig                     |
| `basePermille` | heute    |         9 |    10 | nicht gemeldet — richtig                     |

## Die Zahlen

| Kategorie                                                                     | Anzahl |
| ----------------------------------------------------------------------------- | -----: |
| **nie gelesen** (kein Treffer ausserhalb der Definition, auch nicht in Tests) |     55 |
| **nur in Prüffällen gelesen**                                                 |     99 |

Die zweite Kategorie ist die gefährlichere: Sie sieht in der Abdeckung gedeckt
aus. Sie enthält aber auch **legitime Fälle**, die keine toten Achsen sind:

- **Paritäts-Gegenstücke** wie `base_permille` in Rust. Die Rechnung passiert im
  Spiel; die Rust-Fassung existiert, damit die Aufzeichnung sie vergleichen
  kann. Nur in Tests gerufen zu werden ist ihr Zweck.
- **Zugangsfunktionen** wie `boss_by_id` (177 Test-Treffer) — öffentliche API.
- **Prüfzugänge** wie `gcd_length_for_test`, die ich selbst dafür gebaut habe.

Diese Einordnung ist meine Auslegung und steht hier, damit ihr widersprochen
werden kann. Die rohen Zahlen darüber sind gemessen.

## Ein Befund über meine eigene Arbeit

`SET_VALUE_TIERS` steht auf beiden Seiten in der Liste — und **ich habe es
verwaist**. Vor meinem PR #771 („eine Wahrheit für Topf 4") hatte es 4 Treffer
in `sim/src/sources.rs` und 2 in `src/gear/sources.ts`; heute steht nur noch
die Definition da.

Beim Beseitigen eines Doppelwegs ist eine tote Achse entstanden. Genau die
Klasse, gegen die diese Erhebung läuft — begangen in dem Zweig, der sie
beseitigte. Es ist der dritte Fall dieser Art in zwei Tagen.

## Nie gelesen (55)

| Symbol | Datei | Art | entstand | Tests |
| ------ | ----- | --- | -------- | ----: |

| `attachHelp` | `src/analysis/archive-view.ts` | function | 06a4a81 2026-08-04 | 0 |
| `buildArchiveView` | `src/analysis/archive-view.ts` | function | 06a4a81 2026-08-04 | 0 |
| `buildIntroduction` | `src/analysis/archive-view.ts` | function | 06a4a81 2026-08-04 | 0 |
| `SET_VALUE_TIERS` | `sim/src/sources.rs` | const | 0e4528d 2026-08-04 | 0 |
| `buildComparisonView` | `src/analysis/history-view.ts` | function | 10c2831 2026-08-04 | 0 |
| `buildCurveView` | `src/analysis/history-view.ts` | function | 10c2831 2026-08-04 | 0 |
| `buildHistoryView` | `src/analysis/history-view.ts` | function | 10c2831 2026-08-04 | 0 |
| `isOpen` | `src/credits/credits.ts` | method | 22b6b9f 2026-07-27 | 0 |
| `REFERENCE_FIGHT_RUNNING` | `src/devtools/reference-run.ts` | const | 399f2c8 2026-07-28 | 0 |
| `buildEfficiencyView` | `src/analysis/analysis-view.ts` | function | 42f3ff4 2026-08-04 | 0 |
| `HIGHEST_PRESENTATION` | `src/loot/moment.ts` | const | 4ce58b3 2026-08-04 | 0 |
| `currentQuality` | `src/loot/moment.ts` | method | 4ce58b3 2026-08-04 | 0 |
| `shownTotal` | `src/loot/moment.ts` | method | 4ce58b3 2026-08-04 | 0 |
| `stepLoot` | `src/render/overlay.ts` | method | 4ce58b3 2026-08-04 | 0 |
| `SOURCE_LEGENDARY` | `sim/src/sources.rs` | const | 65d3aa0 2026-08-04 | 0 |
| `SOURCE_SETS` | `sim/src/sources.rs` | const | 65d3aa0 2026-08-04 | 0 |
| `SOURCE_TALENTS` | `sim/src/sources.rs` | const | 65d3aa0 2026-08-04 | 0 |
| `isTrackingChecksums` | `src/core/session.ts` | method | 673bd5d 2026-07-28 | 0 |
| `pool_count` | `sim/src/gear_balance.rs` | fn | 78c0bb8 2026-08-04 | 0 |
| `spells_used` | `sim/src/apl/mod.rs` | fn | 86a2f1b 2026-07-28 | 0 |
| `lastHitAt` | `src/render/impact.ts` | method | a578733 2026-07-28 | 0 |
| `EVENT_PLAYER_DEATH` | `src/core/log.ts` | const | a7aa2bd 2026-08-04 | 0 |
| `DIFFICULTY_NAMES` | `src/loot/rates.ts` | const | b0bbba8 2026-08-04 | 0 |
| `EVENT_KILL` | `src/core/log.ts` | const | b8e2813 2026-07-29 | 0 |
| `EVENT_SPEND` | `src/core/log.ts` | const | b8e2813 2026-07-29 | 0 |
| `runLiveMetrics` | `src/analysis/training-view.ts` | function | c549371 2026-08-04 | 0 |
| `BLOCK_HEIGHT` | `src/render/spellbar.ts` | const | c7d7656 2026-07-28 | 0 |
| `best_affix_build` | `sim/src/best_build.rs` | fn | c939e9e 2026-08-04 | 0 |
| `tipping_set_bonus` | `sim/src/best_build.rs` | fn | c939e9e 2026-08-04 | 0 |
| `isVisible` | `src/progression/player-home.ts` | method | cb554a2 2026-08-04 | 0 |
| `sectionIds` | `src/progression/player-home.ts` | method | cb554a2 2026-08-04 | 0 |
| `set_build` | `sim/src/sources.rs` | fn | d690acc 2026-08-04 | 0 |
| `POOL_ACTIVE` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 0 |
| `POOL_YIELD` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 0 |
| `QUALITY_LEGENDARY` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 0 |
| `QUALITY_RARE` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 0 |
| `POOLS_OF_SPECIAL_STATS` | `src/gear/pools.ts` | const | d93660c 2026-07-29 | 0 |
| `label` | `sim/src/rotation_search.rs` | fn | db08daf 2026-08-05 | 0 |
| `lastFailure` | `src/core/audio-bridge.ts` | method | e0f28c2 2026-07-27 | 0 |
| `ROW_COUNT` | `src/input/keys.ts` | const | e0f28c2 2026-07-27 | 0 |
| `resource_data` | `sim/src/fight.rs` | fn | e260dcb 2026-07-28 | 0 |
| `dummyFor` | `src/analysis/training.ts` | method | e4be979 2026-08-04 | 0 |
| `VESSEL_INK` | `src/render/vessel.ts` | const | f5987e2 2026-07-28 | 0 |
| `VESSEL_INK_DIM` | `src/render/vessel.ts` | const | f5987e2 2026-07-28 | 0 |
| `reseed` | `src/audio/rng.ts` | method | f77ec4e 2026-07-27 | 0 |
| `wantedLayers` | `src/core/audio-bridge.ts` | method | f7df229 2026-07-28 | 0 |
| `EV_BOSS_CAST_START` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `EV_BOSS_CAST_SUCCESS` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `EV_BOSS_PHASE_CHANGE` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `EV_BOSS_REACTION_DESTABILISE` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `EV_PLAYER_LOW_HEALTH` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `EV_SYNERGY_READY` | `src/core/events.ts` | const | f7df229 2026-07-28 | 0 |
| `bestTimeMs` | `src/core/session.ts` | method | f7df229 2026-07-28 | 0 |
| `scaling` | `sim/src/balance.rs` | fn | fa83159 2026-07-29 | 0 |
| `lastExpiredKind` | `src/combat/schools/buffs.ts` | method | fcf5661 2026-07-29 | 0 |

## Nur in Prüffällen gelesen (99)

| Symbol | Datei | Art | entstand | Tests |
| ------ | ----- | --- | -------- | ----: |

| `compareRuns` | `src/analysis/run-compare.ts` | function | 04f03e4 2026-08-04 | 8 |
| `sampleDamageCurve` | `src/analysis/run-compare.ts` | function | 04f03e4 2026-08-04 | 7 |
| `markSeen` | `src/analysis/help.ts` | function | 0744f47 2026-08-04 | 2 |
| `missingSubjects` | `src/analysis/help.ts` | function | 0744f47 2026-08-04 | 3 |
| `nextIntroduction` | `src/analysis/help.ts` | function | 0744f47 2026-08-04 | 4 |
| `is_dead` | `sim/src/report.rs` | fn | 1473df2 2026-07-29 | 4 |
| `boss_by_id` | `sim/src/data.rs` | fn | 1fdff82 2026-07-29 | 177 |
| `validateCredits` | `src/credits/credits.ts` | function | 22b6b9f 2026-07-27 | 8 |
| `cast_count` | `sim/src/fight.rs` | fn | 25e0369 2026-07-28 | 16 |
| `stream_seed` | `sim/src/rng.rs` | fn | 25e0369 2026-07-28 | 1 |
| `recordFieldOrder` | `src/core/checksum.ts` | function | 33186cf 2026-08-05 | 8 |
| `STREAM_NAMES` | `src/core/rng.ts` | const | 399f2c8 2026-07-28 | 6 |
| `boundSlotCount` | `src/core/session.ts` | method | 399f2c8 2026-07-28 | 5 |
| `sumOf` | `src/core/log.ts` | method | 46b4e26 2026-07-28 | 1 |
| `gcd_length_for_test` | `sim/src/fight.rs` | fn | 4aa51d1 2026-08-11 | 2 |
| `IGNORED_OUTCOMES` | `src/analysis/run-recorder.ts` | const | 4adc9aa 2026-08-04 | 2 |
| `BUFF_KIND_MASK_LIMIT` | `src/combat/schools/buffs.ts` | const | 4cb3658 2026-08-09 | 4 |
| `isMarkedAsApproximation` | `src/loot/compare.ts` | function | 4ce58b3 2026-08-04 | 4 |
| `presentationsAreDistinct` | `src/loot/moment.ts` | function | 4ce58b3 2026-08-04 | 2 |
| `skippedTotal` | `src/loot/moment.ts` | method | 4ce58b3 2026-08-04 | 2 |
| `materialFor` | `src/loot/upgrade.ts` | function | 4ce58b3 2026-08-04 | 11 |
| `upgradeCapOf` | `src/loot/upgrade.ts` | function | 4ce58b3 2026-08-04 | 4 |
| `base_permille` | `sim/src/sources.rs` | fn | 51c9d46 2026-08-05 | 2 |
| `unlockedSlots` | `src/progression/unlocks.ts` | function | 521652d 2026-08-05 | 5 |
| `gainCount` | `src/combat/schools/stacks.ts` | method | 5716923 2026-07-29 | 8 |
| `ALLOWED_TOP_LEVEL` | `src/analysis/save-format.ts` | const | 57e9840 2026-07-29 | 2 |
| `newLevelState` | `src/progression/level.ts` | function | 61a7a5b 2026-08-05 | 7 |
| `NO_SUBJECT` | `src/combat/events.ts` | const | 6476d73 2026-07-29 | 4 |
| `RULE_BOSS_PHASE` | `src/combat/events.ts` | const | 6476d73 2026-07-29 | 5 |
| `RULE_BUFF_EXPIRED` | `src/combat/events.ts` | const | 6476d73 2026-07-29 | 5 |
| `receiverCount` | `src/combat/events.ts` | method | 6476d73 2026-07-29 | 1 |
| `SET_VALUE_TIERS` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 3 |
| `SOURCE_LEGENDARY` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 4 |
| `SOURCE_NAMES` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 14 |
| `SOURCE_SETS` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 4 |
| `SOURCE_TALENTS` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 6 |
| `STAT_NAMES` | `src/gear/sources.ts` | const | 65d3aa0 2026-08-04 | 10 |
| `buildTotal` | `src/gear/sources.ts` | function | 65d3aa0 2026-08-04 | 7 |
| `mixedBuild` | `src/gear/sources.ts` | function | 65d3aa0 2026-08-04 | 7 |
| `totalMax` | `src/gear/sources.ts` | function | 65d3aa0 2026-08-04 | 9 |
| `estimatedGain` | `src/loot/estimate.ts` | function | 71823dd 2026-08-06 | 3 |
| `stack_gain_count` | `sim/src/fight.rs` | fn | 723aeaf 2026-07-29 | 4 |
| `resetArmedBy` | `src/core/session.ts` | method | 7896d07 2026-07-29 | 1 |
| `resetsAppliedCount` | `src/core/session.ts` | method | 7896d07 2026-07-29 | 9 |
| `resetsWastedCount` | `src/core/session.ts` | method | 7896d07 2026-07-29 | 7 |
| `tiersReached` | `src/gear/sets.ts` | function | 7dd9702 2026-08-04 | 12 |
| `rule_events` | `sim/src/fight.rs` | fn | 889baa0 2026-07-29 | 2 |
| `coverageTotals` | `src/audio/manifest.ts` | function | 8b6e368 2026-07-27 | 2 |
| `applicationCount` | `src/combat/schools/sublimation.ts` | method | 9973def 2026-07-29 | 4 |
| `shortenedCount` | `src/combat/schools/sublimation.ts` | method | 9973def 2026-07-29 | 5 |
| `self_cancelled_count` | `sim/src/fight.rs` | fn | a749348 2026-08-10 | 1 |
| `EVENT_CONVERTED_TO_SHIELD` | `src/core/log.ts` | const | a7aa2bd 2026-08-04 | 3 |
| `DIFFICULTY_HEROIC` | `src/loot/rates.ts` | const | b0bbba8 2026-08-04 | 3 |
| `DIFFICULTY_MYTHIC` | `src/loot/rates.ts` | const | b0bbba8 2026-08-04 | 16 |
| `DIFFICULTY_NORMAL` | `src/loot/rates.ts` | const | b0bbba8 2026-08-04 | 17 |
| `sublimation_use_count` | `sim/src/fight.rs` | fn | bb40829 2026-07-29 | 4 |
| `receiver_ids` | `sim/src/rules_event.rs` | fn | c62b7a3 2026-07-29 | 6 |
| `fadeOutAll` | `src/audio/music.ts` | method | cb8dcac 2026-07-28 | 1 |
| `isEnabled` | `src/audio/music.ts` | method | cb8dcac 2026-07-28 | 1 |
| `POOL_NAMES` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 3 |
| `QUALITY_NAMES` | `sim/src/gear.rs` | const | d93660c 2026-07-29 | 2 |
| `isDistinguishable` | `src/gear/format.ts` | function | d93660c 2026-07-29 | 3 |
| `POOL_ACTIVE` | `src/gear/pools.ts` | const | d93660c 2026-07-29 | 2 |
| `POOL_ESSENCE` | `src/gear/pools.ts` | const | d93660c 2026-07-29 | 8 |
| `POOL_GEAR_EFFECTS` | `src/gear/pools.ts` | const | d93660c 2026-07-29 | 27 |
| `SLOT_ANCHOR` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_CATALYST` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_FOCUS` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_GRIMOIRE` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_PHYLACTERY` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_PRISM` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_RELIC` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_SHARD` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_SIGIL` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `SLOT_TALISMAN` | `src/gear/slots.ts` | const | d93660c 2026-07-29 | 3 |
| `exhaustive` | `sim/src/rotation_search.rs` | fn | db08daf 2026-08-05 | 4 |
| `isDefeated` | `src/analysis/defeated.ts` | function | db149b2 2026-08-06 | 6 |
| `isNothingWorn` | `src/gear/worn.ts` | function | e0d11dd 2026-08-04 | 5 |
| `codeOfSlot` | `src/input/keys.ts` | function | e0f28c2 2026-07-27 | 2 |
| `isSlot` | `src/input/keys.ts` | function | e0f28c2 2026-07-27 | 6 |
| `layerOfSlot` | `src/input/keys.ts` | function | e0f28c2 2026-07-27 | 2 |
| `isHeld` | `src/input/router.ts` | method | e0f28c2 2026-07-27 | 2 |
| `dot_error_only` | `sim/src/rotation.rs` | fn | e260dcb 2026-07-28 | 4 |
| `pyre_error_only` | `sim/src/rotation.rs` | fn | e260dcb 2026-07-28 | 4 |
| `setLoadout` | `src/analysis/training.ts` | method | e4be979 2026-08-04 | 4 |
| `setSettings` | `src/analysis/training.ts` | method | e4be979 2026-08-04 | 2 |
| `worstFrameMs` | `src/core/metrics.ts` | method | e4fd868 2026-07-27 | 2 |
| `overwritten` | `src/core/ring.ts` | method | e4fd868 2026-07-27 | 1 |
| `setSetEffects` | `src/core/combat.ts` | method | e6c8460 2026-08-11 | 5 |
| `nextAutomaticEntry` | `src/analysis/archive.ts` | function | efdb69e 2026-07-29 | 5 |
| `unlockedCount` | `src/analysis/archive.ts` | function | efdb69e 2026-07-29 | 3 |
| `isFinished` | `src/core/phases.ts` | function | f5987e2 2026-07-28 | 3 |
| `isVictory` | `src/core/phases.ts` | function | f5987e2 2026-07-28 | 3 |
| `historyFor` | `src/analysis/run-history.ts` | function | fc33351 2026-08-04 | 3 |
| `expiration_count` | `sim/src/buffs.rs` | fn | fcf5661 2026-07-29 | 1 |
| `expirationCount` | `src/combat/schools/buffs.ts` | method | fcf5661 2026-07-29 | 2 |
| `replacementCount` | `src/combat/schools/buffs.ts` | method | fcf5661 2026-07-29 | 4 |
| `hiddenIndicatorCount` | `src/combat/schools/indicators.ts` | function | fcf5661 2026-07-29 | 4 |
| `to_json` | `sim/src/balance.rs` | fn | ffe03da 2026-07-29 | 1 |

## Was diese Liste NICHT sagt

- **Kein Urteil über Absicht.** Manches ist bewusst vorgehalten (`mayDropAt`
  stand laut Kommentar absichtlich vor dem ersten gebundenen Stück da).
- **Namensgleichheit** kann Treffer erzeugen: Ein kurzer Methodenname, den es
  in mehreren Klassen gibt, wird gemeinsam gezählt. Für die grossen Einträge
  habe ich stichprobenartig nachgesehen; für alle 154 nicht.
- **Nicht geprüft: dynamische Aufrufe.** Wird ein Symbol über eine Tabelle oder
  einen String gerufen, sieht das Werkzeug es nicht.
