/**
 * Schreibt die gemeinsamen Vergleichsdateien für den Abgleich mit dem Simulator.
 *
 * ```text
 * npm run parity:export
 * ```
 *
 * Der Eintrag in `package.json` ist seit dem 2026-08-05 da, und zwar aus einem
 * Grund, der über Bequemlichkeit hinausgeht: Der Befehl wurde in einer Nacht
 * zweimal gesucht und stand beide Male nirgends, wo man sucht. Ein Skript kann
 * nicht veralten wie ein Kommentar — dieser hier hatte es getan.
 *
 * Direkt geht es auch, und dann zeigt sich, warum es `vite-node` sein muss:
 *
 * ```text
 * npx vite-node tests/parity/export-parity.mjs
 * ```
 *
 * **`npx vite-node`, nicht `node` und nicht `npx tsx`.** Beide scheitern, und
 * zwar verschieden — beides am 2026-08-05 gemessen:
 *
 * | Aufruf | Fehler |
 * | --- | --- |
 * | `node …` | `ERR_MODULE_NOT_FOUND`: sucht `src/core/checksum.js`, da liegt nur die `.ts` |
 * | `npx tsx …` | `TypeError: (intermediate value).glob is not a function` |
 *
 * Das zweite ist das lehrreichere: `tsx` versteht TypeScript, aber
 * `import.meta.glob` ist keine Sprache, sondern Vite — und daran hängt das
 * Einlesen der Boss- und Zauber-Daten. Der Dateiname stand hier bis dahin mit
 * der Endung `.ts`; die Datei heißt aber `.mjs`, und der abgetippte Befehl lief
 * deshalb ins Leere.
 *
 * Alle Vergleichsdateien unter `tests/parity/recordings/` entstehen dabei neu,
 * nicht nur die eine, an der man gerade arbeitet. Nach dem Lauf lohnt ein
 * `git status`: Ändert sich mehr als erwartet, hat sich eine Regel bewegt.
 * Ein paar davon zur Einordnung:
 *
 * | Datei | Inhalt |
 * | --- | --- |
 * | `cooldown-precision.json` | Eine aufgezeichnete Runde samt Ergebnis und Prüfsumme je Takt |
 * | `primitives.json` | Zufallsströme und Prüfsumme — die Bausteine darunter |
 * | `rule-events.json` | Eine feste Folge von Regel-Ereignissen samt Prüfsumme |
 * | `trial-blocks.json` | Die letzten drei Boss-Bausteine in einem Lauf (#240) |
 *
 * **Nach jeder Regeländerung neu laufen lassen.** `freshness.test.ts` erzwingt
 * das: Er baut dieselben Inhalte und vergleicht sie mit dem, was hier liegt.
 *
 * Die Werte sind ausdrücklich *aufgezeichnete echte Durchläufe* (GAME-DESIGN
 * 18.4) — sie entstehen aus dem laufenden Spielcode, nicht aus abgetippten
 * Listen. Wer eine Regel ändert, sieht im Diff, was sich bewegt hat; das ist der
 * eigentliche Zweck.
 *
 * **Warum JavaScript und nicht TypeScript:** Diese Datei ist das einzige Stück
 * des Projekts, das ins Dateisystem schreibt, und braucht dafür Node-APIs. Das
 * Projekt führt aber bewusst keine Node-Typen (`tsconfig.json`: `types:
 * ["vite/client"]`) — es ist ein Browser-Projekt. Sie deswegen einzuziehen wäre
 * ein Eingriff in die Konfiguration für vierzig Zeilen Werkzeug. Der Inhalt der
 * Dateien entsteht ohnehin in `payloads.ts` und ist dort typgeprüft; hier wird
 * er nur noch geschrieben.
 */
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  buffsPayload,
  buffCombatPayload,
  decayFactorPayload,
  decayTalentPayload,
  spenderTalentPayload,
  kontrollPreisTalentPayload,
  KONTROLL_PREIS_TALENT_NAME,
  schluesselNachlaufPayload,
  SCHLUESSEL_NACHLAUF_NAME,
  schluesselNachlagePayload,
  SCHLUESSEL_NACHLAGE_NAME,
  schluesselNachlageAblaufPayload,
  SCHLUESSEL_NACHLAGE_ABLAUF_NAME,
  fixationSpellsPayload,
  resetRunPayload,
  raceHealPayload,
  raceLostPayload,
  trialBlocksPayload,
  TRIAL_BLOCKS_NAME,
  gearPayload,
  hastePayload,
  wornPayload,
  sourcesPayload,
  WORN_NAME,
  setsPayload,
  logKindsPayload,
  bossStatePayload,
  doomStacksPayload,
  doomPurgePayload,
  cooldownResetPayload,
  misplaysPayload,
  synergyPayload,
  SYNERGY_NAME,
  synergyHastePayload,
  SYNERGY_HASTE_NAME,
  wardCapPayload,
  WARD_CAP_NAME,
  rotPayload,
  ROT_NAME,
  putrefactionFillerPayload,
  PUTREFACTION_FILLER_NAME,
  buffTargetsPayload,
  BUFF_TARGETS_NAME,
  capStapelungPayload,
  CAP_STAPELUNG_NAME,
  tickRundungPayload,
  TICK_RUNDUNG_NAME,
  instance1Payload,
  instance2Payload,
  instance3Payload,
  instance4Payload,
  instance5Payload,
  INSTANCE2_NAMES,
  INSTANCE1_NAMES,
  INSTANCE3_NAMES,
  INSTANCE4_NAMES,
  INSTANCE5_NAMES,
  cinderPayload,
  dotLifetimePayload,
  manyDotsPayload,
  DOT_LIFETIME_NAME,
  MANY_DOTS_NAME,
  extendEffectsPayload,
  EXTEND_EFFECTS_NAME,
  COAGULATION_NEU_NAME,
  coagulationNeuPayload,
  kernePayload,
  KERNE_NAME,
  pinPayload,
  PIN_NAME,
  decayWardPayload,
  DECAY_WARD_NAME,
  putridBloomPayload,
  PUTRID_BLOOM_NAME,
  effluxPayload,
  EFFLUX_NAME,
  kritPayload,
  KRIT_NAME,
  CINDER_NAME,
  playerDamagePayload,
  coagulationPayload,
  kontrolleWirktPayload,
  fixationPayload,
  logParityPayload,
  logTaktePayload,
  LOG_TAKTE_NAME,
  aetherPayload,
  AETHER_NAME,
  sublimePayload,
  SUBLIME_NAME,
  gcdFreiPayload,
  GCD_FREI_NAME,
  strikterUmschlagPayload,
  STRIKT_NAME,
  condensePayload,
  CONDENSE_NAME,
  retortPayload,
  RETORT_NAME,
  kontrollFehlschlagPayload,
  KONTROLL_FEHLSCHLAG_NAME,
  timelineLoopPayload,
  sublimationPayload,
  gezielteVerkuerzungPayload,
  GEZIELTE_VERKUERZUNG_NAME,
  SUBLIMATION_NAME,
  wispPayload,
  WISP_NAME,
  alembicPayload,
  ALEMBIC_NAME,
  deckelPayload,
  DECKEL_NAME,
  phasenPayload,
  PHASEN_NAME,
  productionPhasesPayload,
  PRODUCTION_PHASES_NAME,
  TIMELINE_LOOP_NAME,
  primitivesPayload,
  recordingPayload,
  ruleEventsPayload,
  MISPLAY_NAME,
  RECORDING_NAME,
  VESSEL_NAME,
  DOOM_STACKS_NAME,
  DOOM_PURGE_NAME,
  RESET_RUN_NAME,
  RACE_HEAL_NAME,
  RACE_LOST_NAME,
  PLAYER_DAMAGE_NAME,
  COAGULATION_NAME,
  KONTROLLE_WIRKT_NAME,
  FIXATION_NAME,
  LOG_PARITY_NAME,
  serialize,
  RUNAWAY_PROGRESS_NAME,
  runawayProgressPayload,
  LEVEL_30_NAME,
  level30Payload,
  CAST_TIME_NAME,
  CHANNEL_NAME,
  CHANNEL_SHIELD_NAME,
  CHANNEL_HEAL_NAME,
  castTimePayload,
  channelPayload,
  channelShieldPayload,
  channelHealPayload,
  RESISTANCE_NAME,
  resistancePayload,
  RESISTANCE_MIXED_NAME,
  resistanceMixedPayload,
  newCalcinationPayload,
  NEW_CALCINATION_NAME,
} from './payloads.ts';
import { overdrivePayload, OVERDRIVE_NAME } from './overdrive-payload.ts';

const here = dirname(fileURLToPath(import.meta.url));

const newCalcination = newCalcinationPayload();
const newCalcinationPath = join(here, 'recordings', `${NEW_CALCINATION_NAME}.json`);
writeFileSync(newCalcinationPath, serialize(newCalcination), 'utf8');

const recording = recordingPayload();
const recordingPath = join(here, 'recordings', `${RECORDING_NAME}.json`);
writeFileSync(recordingPath, serialize(recording), 'utf8');

const primitives = primitivesPayload();
const primitivesPath = join(here, 'recordings', 'primitives.json');
writeFileSync(primitivesPath, serialize(primitives), 'utf8');

const misplays = misplaysPayload();
const misplaysPath = join(here, 'recordings', `${MISPLAY_NAME}.json`);
writeFileSync(misplaysPath, serialize(misplays), 'utf8');

const synergy = synergyPayload();
const synergyPath = join(here, 'recordings', `${SYNERGY_NAME}.json`);
writeFileSync(synergyPath, serialize(synergy), 'utf8');

// Tempo UND Synergie zusammen (Issue #456, Schritt 2). Ohne diese Datei konnten
// beide Seiten verschieden klammern, ohne dass die Paritaetspruefung es sah.
const synergyHaste = synergyHastePayload();
const synergyHastePath = join(here, 'recordings', `${SYNERGY_HASTE_NAME}.json`);
writeFileSync(synergyHastePath, serialize(synergyHaste), 'utf8');

// Der WARD-Deckel (Issue #466). Er ist eine REGEL, keine Zahl in den Daten —
// ohne eigene Aufzeichnung waere er auf beiden Seiten eingebaut und auf keiner
// verglichen.
const wardCap = wardCapPayload();
const wardCapPath = join(here, 'recordings', `${WARD_CAP_NAME}.json`);
writeFileSync(wardCapPath, serialize(wardCap), 'utf8');

// ROT ist der erste Zauber, der Zersetzungs-Stapel ohne Tastendruck legt
// (Issue #498). Ohne eigene Aufzeichnung gaelte seine Bitgleichheit als
// angenommen — `coverage.test.ts` nennt ihn dann beim Namen.
const rot = rotPayload();
const rotPath = join(here, 'recordings', `${ROT_NAME}.json`);
writeFileSync(rotPath, serialize(rot), 'utf8');
// BLIGHT im Paritaets-Weg (Issue #479). Ohne ihn gilt die Bitgleichheit fuer
// den neuen Zauber als angenommen, nicht als geprueft.
const putrefactionFiller = putrefactionFillerPayload();
const putrefactionFillerPath = join(here, 'recordings', `${PUTREFACTION_FILLER_NAME}.json`);
writeFileSync(putrefactionFillerPath, serialize(putrefactionFiller), 'utf8');

// CINDER im Paritaets-Weg (Issue #477). Ohne ihn gilt seine Bitgleichheit als
// angenommen — er war nach Worker-1s Arbeit der einzige ungedeckte Zauber.
const cinder = cinderPayload();
const cinderPath = join(here, 'recordings', `${CINDER_NAME}.json`);
writeFileSync(cinderPath, serialize(cinder), 'utf8');

// EMBER und PYROCLASM (#517) sowie HARDEN und SEPSIS (#523). Vor diesen vier
// Zeilen wirkte KEINE Aufzeichnung sie — ihre Bitgleichheit galt fuer alle
// vier als angenommen, und `coverage.test.ts` stand deshalb auf rot.
const dotLifetime = dotLifetimePayload();
const dotLifetimePath = join(here, 'recordings', `${DOT_LIFETIME_NAME}.json`);
writeFileSync(dotLifetimePath, serialize(dotLifetime), 'utf8');

// EMBER auf DREI Braende zugleich (#485, AC3). `dot-lifetime` faehrt nur einen
// — die Gegenprobe "verlaengere nur den ersten" lief dort durch alle 134
// Rust-Suiten, ohne eine einzige rot zu faerben.
const manyDots = manyDotsPayload();
const manyDotsPath = join(here, 'recordings', `${MANY_DOTS_NAME}.json`);
writeFileSync(manyDotsPath, serialize(manyDots), 'utf8');

// Die neuen Zauber der COAGULATION (#481). Ohne sie waere ihre
// Bitgleichheit eine Annahme — `coverage` sagt es woertlich.
const coagulationNeu = coagulationNeuPayload();
const coagulationNeuPath = join(here, 'recordings', `${COAGULATION_NEU_NAME}.json`);
writeFileSync(coagulationNeuPath, serialize(coagulationNeu), 'utf8');

const extendEffects = extendEffectsPayload();
const extendEffectsPath = join(here, 'recordings', `${EXTEND_EFFECTS_NAME}.json`);
writeFileSync(extendEffectsPath, serialize(extendEffects), 'utf8');

// Die fuenf Verstaerkungen aus #540. Ohne sie wirkt KEINE Aufzeichnung
// flashpoint, quicksilver, distil, griplock oder virulence — ihre
// Bitgleichheit waere eine Annahme.
const buffTargets = buffTargetsPayload();
const buffTargetsPath = join(here, 'recordings', `${BUFF_TARGETS_NAME}.json`);
writeFileSync(buffTargetsPath, serialize(buffTargets), 'utf8');

// Zwei Verstaerkungen VERSCHIEDENER Art auf derselben Zielgroesse (#562).
// Die erste Lage dieser Art im Bestand: Vorher trug nur SLAG `kind: 1`, und
// er sass allein im Speicher `cap`. Ohne diese Aufzeichnung ist die
// Bitgleichheit der Stapel-Regel eine Annahme.
const capStapelung = capStapelungPayload();
const capStapelungPath = join(here, 'recordings', `${CAP_STAPELUNG_NAME}.json`);
writeFileSync(capStapelungPath, serialize(capStapelung), 'utf8');

// Der Tick-Abstand unter Verstaerkung UND Tempo (#640). `tick` war die einzige
// Zielgroesse ohne Traeger in einer Aufzeichnung — und die einzige, deren
// Faktor TEILT statt zu multiplizieren.
const tickRundung = tickRundungPayload();
const tickRundungPath = join(here, 'recordings', `${TICK_RUNDUNG_NAME}.json`);
writeFileSync(tickRundungPath, serialize(tickRundung), 'utf8');

// Die vier Bosse der ersten Instanz (#544, AC4).
for (let i = 0; i < INSTANCE1_NAMES.length; i++) {
  const name = INSTANCE1_NAMES[i];
  const datensatz = instance1Payload(`boss-1-${i + 1}`, name);
  writeFileSync(join(here, 'recordings', `${name}.json`), serialize(datensatz), 'utf8');
}

// Die vier Bosse der zweiten Instanz (#574, AC4).
for (let i = 0; i < INSTANCE2_NAMES.length; i++) {
  const name = INSTANCE2_NAMES[i];
  const datensatz = instance2Payload(`boss-2-${i + 1}`, name);
  writeFileSync(join(here, 'recordings', `${name}.json`), serialize(datensatz), 'utf8');
}

// Die vier Bosse der dritten Instanz. Sie hatten als einzige Instanz KEINE
// Aufzeichnung — siehe die Begruendung bei `INSTANCE3_NAMES` in `payloads.ts`.
for (let i = 0; i < INSTANCE3_NAMES.length; i++) {
  const name = INSTANCE3_NAMES[i];
  const datensatz = instance3Payload(`boss-3-${i + 1}`, name);
  writeFileSync(join(here, 'recordings', `${name}.json`), serialize(datensatz), 'utf8');
  console.log(
    `geschrieben: ${name}  Schaden ${datensatz.expected.damage}, ` +
      `gefallen ${datensatz.expected.killedAtMs}, gestorben ${datensatz.expected.diedAtMs}, ` +
      `Druecke ${datensatz.presses.length}`,
  );
}

// Die vier Bosse der vierten Instanz. Sie entstehen, weil der Waechter
// `jeder-boss-hat-eine-aufzeichnung` sie eingefordert hat — siehe payloads.ts.
for (let i = 0; i < INSTANCE4_NAMES.length; i++) {
  const name = INSTANCE4_NAMES[i];
  const datensatz = instance4Payload(`boss-4-${i + 1}`, name);
  writeFileSync(join(here, 'recordings', `${name}.json`), serialize(datensatz), 'utf8');
  console.log(
    `geschrieben: ${name}  Schaden ${datensatz.expected.damage}, ` +
      `gefallen ${datensatz.expected.killedAtMs}, gestorben ${datensatz.expected.diedAtMs}, ` +
      `Druecke ${datensatz.presses.length}`,
  );
}

// Die vier Bosse der fünften Instanz. Derselbe Grund wie bei der vierten — der
// Wächter hat sie am Tag ihres Eintrags beim Namen genannt.
for (let i = 0; i < INSTANCE5_NAMES.length; i++) {
  const name = INSTANCE5_NAMES[i];
  const datensatz = instance5Payload(`boss-5-${i + 1}`, name);
  writeFileSync(join(here, 'recordings', `${name}.json`), serialize(datensatz), 'utf8');
  console.log(
    `geschrieben: ${name}  Schaden ${datensatz.expected.damage}, ` +
      `gefallen ${datensatz.expected.killedAtMs}, gestorben ${datensatz.expected.diedAtMs}, ` +
      `Druecke ${datensatz.presses.length}`,
  );
}

const kerne = kernePayload();
const kernePath = join(here, 'recordings', `${KERNE_NAME}.json`);
writeFileSync(kernePath, `${JSON.stringify(kerne, null, 2)}\n`);
console.log(`geschrieben: ${kernePath}`);
console.log(`  gewirkt   ${JSON.stringify(kerne.expected.casts)}`);

const pin = pinPayload();
const pinPath = join(here, 'recordings', `${PIN_NAME}.json`);
writeFileSync(pinPath, `${JSON.stringify(pin, null, 2)}\n`);
console.log(`geschrieben: ${pinPath}`);
console.log(`  gewirkt   ${JSON.stringify(pin.expected.casts)}`);
console.log(`  erlitten  ${pin.expected.damageTaken}`);
const decayWard = decayWardPayload();
const decayWardPath = join(here, 'recordings', `${DECAY_WARD_NAME}.json`);
writeFileSync(decayWardPath, `${JSON.stringify(decayWard, null, 2)}\n`);
console.log(`geschrieben: ${decayWardPath}`);
console.log(`  erlitten  ${decayWard.expected.damageTaken}`);
const bloom = putridBloomPayload();
const bloomPath = join(here, 'recordings', `${PUTRID_BLOOM_NAME}.json`);
writeFileSync(bloomPath, `${JSON.stringify(bloom, null, 2)}\n`);
console.log(`geschrieben: ${bloomPath}`);
console.log(`  Schaden   ${bloom.expected.damage}`);

const bossState = bossStatePayload();
const bossStatePath = join(here, 'recordings', `${VESSEL_NAME}.json`);
writeFileSync(bossStatePath, serialize(bossState), 'utf8');

const doomStacks = doomStacksPayload();
const doomStacksPath = join(here, 'recordings', `${DOOM_STACKS_NAME}.json`);
writeFileSync(doomStacksPath, serialize(doomStacks), 'utf8');

// Der Zaehler, BEDIENT (Issue #561, AC4). `doom-stacks` daneben laesst ihn
// volllaufen.
const doomPurge = doomPurgePayload();
const doomPurgePath = join(here, 'recordings', `${DOOM_PURGE_NAME}.json`);
writeFileSync(doomPurgePath, serialize(doomPurge), 'utf8');

const resetRun = resetRunPayload();
const resetRunPath = join(here, 'recordings', `${RESET_RUN_NAME}.json`);
writeFileSync(resetRunPath, serialize(resetRun), 'utf8');

const raceHeal = raceHealPayload();
const raceHealPath = join(here, 'recordings', `${RACE_HEAL_NAME}.json`);
writeFileSync(raceHealPath, serialize(raceHeal), 'utf8');

// Die verlorene Richtung des Wettrennens (Issue #651). `race-heal` oben deckt
// nur die gewonnene ab.
const raceLost = raceLostPayload();
const raceLostPath = join(here, 'recordings', `${RACE_LOST_NAME}.json`);
writeFileSync(raceLostPath, serialize(raceLost), 'utf8');

const trialBlocks = trialBlocksPayload();
const trialBlocksPath = join(here, 'recordings', `${TRIAL_BLOCKS_NAME}.json`);
writeFileSync(trialBlocksPath, serialize(trialBlocks), 'utf8');

const level30 = level30Payload();
const level30Path = join(here, 'recordings', `${LEVEL_30_NAME}.json`);
writeFileSync(level30Path, serialize(level30), 'utf8');
console.log(`geschrieben: ${level30Path}`);

const efflux = effluxPayload();
const effluxPath = join(here, 'recordings', `${EFFLUX_NAME}.json`);
writeFileSync(effluxPath, `${JSON.stringify(efflux, null, 2)}\n`);
console.log(`geschrieben: ${effluxPath}`);
console.log(
  `  gewirkt   efflux ${efflux.expected.casts.efflux}, Schaden ${efflux.expected.damage}`,
);

const krit = kritPayload();
const kritPath = join(here, 'recordings', `${KRIT_NAME}.json`);
writeFileSync(kritPath, `${JSON.stringify(krit, null, 2)}\n`);
console.log(`geschrieben: ${kritPath}`);
console.log(`  gewirkt   wedge ${krit.expected.casts.wedge}, bind ${krit.expected.casts.bind}`);
console.log(
  `  Stufe 30      vessel ${level30.player.vesselAtLevel} statt ${level30.player.vessel}` +
    `, Schaden ${level30.expected.damage}`,
);

const resistances = resistancePayload();
const resistancePath = join(here, 'recordings', `${RESISTANCE_NAME}.json`);
writeFileSync(resistancePath, serialize(resistances), 'utf8');
console.log(`geschrieben: ${resistancePath}`);
console.log(
  `  Widerstand    ${resistances.expected.damage} statt ` +
    `${resistances.expected.damageWithoutResistance} ohne`,
);

const mixed = resistanceMixedPayload();
const mixedPath = join(here, 'recordings', `${RESISTANCE_MIXED_NAME}.json`);
writeFileSync(mixedPath, serialize(mixed), 'utf8');
console.log(`geschrieben: ${mixedPath}`);
console.log(`  gemischt      ${JSON.stringify(mixed.damageBySpell)}`);

const castTime = castTimePayload();
const castTimePath = join(here, 'recordings', `${CAST_TIME_NAME}.json`);
writeFileSync(castTimePath, serialize(castTime), 'utf8');
console.log(`geschrieben: ${castTimePath}`);
console.log(
  `  Wirkzeit      ${castTime.expected.testCastCasts} Wirkungen, ` +
    `${castTime.expected.castsInterrupted} abgebrochen`,
);

// Die Kanalisierung mit Abbruch mitten drin (#323, AC6). Ohne sie waere ihre
// Bitgleichheit eine Annahme — und der Abbruch ist die Stelle, an der sich
// "laufend" von "am Ende" ueberhaupt erst unterscheidet.
const channel = channelPayload();
const channelPath = join(here, 'recordings', `${CHANNEL_NAME}.json`);
writeFileSync(channelPath, serialize(channel), 'utf8');
console.log(`geschrieben: ${channelPath}`);
console.log(
  `  Kanal         ${channel.expected.damage} Schaden, ` +
    `${channel.expected.castsCancelled} abgebrochen`,
);

// Die SCHUTZ-Auszahlung der Kanalisierung (#323, GAME-DESIGN 20.5 Punkt 3).
// Sie stand in beiden Umsetzungen und in keiner Aufzeichnung; eine Mutation im
// Schutz-Zweig des Simulators liess am 2026-08-14 alle 165 Suiten gruen.
const channelShield = channelShieldPayload();
const channelShieldPath = join(here, 'recordings', `${CHANNEL_SHIELD_NAME}.json`);
writeFileSync(channelShieldPath, serialize(channelShield), 'utf8');
console.log(`geschrieben: ${channelShieldPath}`);
console.log(
  `  Kanal-Schutz  ${channelShield.expected.absorbed} abgefangen, ` +
    `${channelShield.shieldTrace.filter((v) => v > 0).length} Takte mit Schicht`,
);

// Die HEIL-Auszahlung der Kanalisierung (#323, Nachtrag zu #843). Eigene Datei
// und anderer Boss, weil ein Schutz-Kanal im selben Lauf die Heilung
// wirkungslos machte — gemessen: 1020 geheilt ohne ihn, 0 mit ihm.
const channelHeal = channelHealPayload();
const channelHealPath = join(here, 'recordings', `${CHANNEL_HEAL_NAME}.json`);
writeFileSync(channelHealPath, serialize(channelHeal), 'utf8');
console.log(`geschrieben: ${channelHealPath}`);
console.log(
  `  Kanal-Heilung ${channelHeal.expected.healed} geheilt, ` +
    `${channelHeal.expected.castsCancelled} von Hand abgebrochen`,
);

const runawayProgress = runawayProgressPayload();
const runawayProgressPath = join(here, 'recordings', `${RUNAWAY_PROGRESS_NAME}.json`);
writeFileSync(runawayProgressPath, serialize(runawayProgress), 'utf8');
console.log(`geschrieben: ${runawayProgressPath}`);
console.log(
  `  Reaktion      Ring ${runawayProgress.expected.damage}, davon vom Spieler ` +
    `${runawayProgress.expected.finalPlayerDamage}`,
);

const playerDamage = playerDamagePayload();
const playerDamagePath = join(here, 'recordings', `${PLAYER_DAMAGE_NAME}.json`);
writeFileSync(playerDamagePath, serialize(playerDamage), 'utf8');

const coagulation = coagulationPayload();
const coagulationPath = join(here, 'recordings', `${COAGULATION_NAME}.json`);
writeFileSync(coagulationPath, serialize(coagulation), 'utf8');

const kontrolleWirkt = kontrolleWirktPayload();
const kontrolleWirktPath = join(here, 'recordings', `${KONTROLLE_WIRKT_NAME}.json`);
writeFileSync(kontrolleWirktPath, serialize(kontrolleWirkt), 'utf8');

const fixation = fixationPayload();
const fixationPath = join(here, 'recordings', `${FIXATION_NAME}.json`);
writeFileSync(fixationPath, serialize(fixation), 'utf8');

const logParity = logParityPayload();
const logParityPath = join(here, 'recordings', `${LOG_PARITY_NAME}.json`);
writeFileSync(logParityPath, serialize(logParity), 'utf8');

const sublime = sublimePayload();
const sublimePath = join(here, 'recordings', `${SUBLIME_NAME}.json`);
writeFileSync(sublimePath, serialize(sublime), 'utf8');
console.log(
  `  SUBLIME-Merker  ${sublime.expected.sublimeCasts} Wuerfe, ` +
    `${sublime.expected.merkerTakte} Merker-Takte, Schaden ${sublime.expected.damageDealt}`,
);

// Nach einem Wurf ohne Wartezeit entscheidet die Liste im naechsten Takt (#1027).
const gcdFrei = gcdFreiPayload();
const gcdFreiPath = join(here, 'recordings', `${GCD_FREI_NAME}.json`);
writeFileSync(gcdFreiPath, serialize(gcdFrei), 'utf8');
console.log(`geschrieben: ${gcdFreiPath}`);
for (const lauf of gcdFrei.runs) {
  console.log(
    `  ${lauf.name.padEnd(24)} Schaden ${lauf.expected.damage}, Leerlauf ${lauf.expected.idleMs} ms`,
  );
}

// Ein strikter Vergleich trifft seinen Umschlag im selben Takt wie das Spiel (#1027).
const strikt = strikterUmschlagPayload();
const striktPath = join(here, 'recordings', `${STRIKT_NAME}.json`);
writeFileSync(striktPath, serialize(strikt), 'utf8');
console.log(`geschrieben: ${striktPath}`);
for (const lauf of strikt.runs) {
  console.log(
    `  ${lauf.name.padEnd(24)} Schaden ${lauf.expected.damage}, Leerlauf ${lauf.expected.idleMs} ms`,
  );
}

const condense = condensePayload();
const condensePath = join(here, 'recordings', `${CONDENSE_NAME}.json`);
writeFileSync(condensePath, serialize(condense), 'utf8');
console.log(
  `  CONDENSE-Merker ${condense.expected.condenseCasts} Wuerfe, ` +
    `${condense.expected.merkerTakte} Merker-Takte, Schaden ${condense.expected.damageDealt}`,
);

const retort = retortPayload();
const retortPath = join(here, 'recordings', `${RETORT_NAME}.json`);
writeFileSync(retortPath, serialize(retort), 'utf8');
console.log(
  `  RETORT          ${retort.expected.retortCasts} Wuerfe, ` +
    `${retort.expected.merkerTakte} geruestete Takte, ` +
    `${retort.expected.einloesungen} eingeloest, Schaden ${retort.expected.damageDealt}`,
);

const kontrollFehlschlag = kontrollFehlschlagPayload();
const kontrollFehlschlagPath = join(here, 'recordings', `${KONTROLL_FEHLSCHLAG_NAME}.json`);
writeFileSync(kontrollFehlschlagPath, serialize(kontrollFehlschlag), 'utf8');
console.log(
  `  Kontroll-Fehlschlag ${kontrollFehlschlag.expected.controlFailed} fehlgeschlagen, ` +
    `${kontrollFehlschlag.expected.controlSpent} Punkte ausgegeben, ` +
    `${kontrollFehlschlag.expected.latchCasts} LATCH / ${kontrollFehlschlag.expected.bindCasts} BIND`,
);

const aether = aetherPayload();
const aetherPath = join(here, 'recordings', `${AETHER_NAME}.json`);
writeFileSync(aetherPath, serialize(aether), 'utf8');
console.log(
  `  AETHER-Fenster  ${aether.expected.aetherCasts} Wuerfe, ` +
    `${aether.expected.wispCasts} WISP, EFFLUX-Restsumme ${aether.expected.effluxRestSumme}`,
);

const logTakte = logTaktePayload();
const logTaktePath = join(here, 'recordings', `${LOG_TAKTE_NAME}.json`);
writeFileSync(logTaktePath, serialize(logTakte), 'utf8');
console.log(
  `  Takte je Art   ${Object.keys(logTakte.takte).length} Arten, ` +
    `${logTakte.expected.bossEnergyTriggers} Energie-Ausloeser`,
);

const sublimation = sublimationPayload();
const sublimationPath = join(here, 'recordings', `${SUBLIMATION_NAME}.json`);
writeFileSync(sublimationPath, serialize(sublimation), 'utf8');

const phasen = phasenPayload();
const phasenPath = join(here, 'recordings', `${PHASEN_NAME}.json`);
writeFileSync(phasenPath, serialize(phasen), 'utf8');

const productionPhases = productionPhasesPayload();
const productionPhasesPath = join(here, 'recordings', `${PRODUCTION_PHASES_NAME}.json`);
writeFileSync(productionPhasesPath, serialize(productionPhases), 'utf8');
console.log(`geschrieben: ${phasenPath}`);
console.log(
  `  Phasen-Bindung         hoechste Phase ${phasen.expected.hoechstePhase}, ` +
    `eingesteckt ${phasen.expected.damageTaken}`,
);

const deckel = deckelPayload();
const deckelPath = join(here, 'recordings', `${DECKEL_NAME}.json`);
writeFileSync(deckelPath, serialize(deckel), 'utf8');
console.log(`geschrieben: ${deckelPath}`);
console.log(
  `  Deckel gefallen        hoechster Vorrat ${deckel.expected.hoechsterVorrat}, ` +
    `Ueberlauf ${deckel.expected.resourceOverflow}`,
);

const alembic = alembicPayload();
const alembicPath = join(here, 'recordings', `${ALEMBIC_NAME}.json`);
writeFileSync(alembicPath, serialize(alembic), 'utf8');
console.log(`geschrieben: ${alembicPath}`);
console.log(
  `  ALEMBIC   ${alembic.expected.alembicCasts} Wurf, ` +
    `${alembic.expected.auswahlen} greifende Auswahlen`,
);
// Die gezielten Verkuerzer (Issue #518) — eigene Aufzeichnung, weil die
// Sublimations-Aufzeichnung ihren eigenen Zweck hat und mit sieben Zaubern
// keinen mehr zeigte.
const gezielteVerkuerzung = gezielteVerkuerzungPayload();
const gezielteVerkuerzungPath = join(here, 'recordings', `${GEZIELTE_VERKUERZUNG_NAME}.json`);
writeFileSync(gezielteVerkuerzungPath, serialize(gezielteVerkuerzung), 'utf8');
console.log(`geschrieben: ${gezielteVerkuerzungPath}`);
console.log(`  gezielt gewirkt ${JSON.stringify(gezielteVerkuerzung['expected']['casts'])}`);

const wisp = wispPayload();
const wispPath = join(here, 'recordings', `${WISP_NAME}.json`);
writeFileSync(wispPath, serialize(wisp), 'utf8');
console.log(`geschrieben: ${wispPath}`);
console.log(
  `  WISP-Verkuerzung       ${wisp.expected.wispCasts} Wuerfe, davon ` +
    `${wisp.expected.wirkendeWuerfe} auf eine laufende VAPOUR-Abklingzeit`,
);

const timelineLoop = timelineLoopPayload();
const timelineLoopPath = join(here, 'recordings', `${TIMELINE_LOOP_NAME}.json`);
writeFileSync(timelineLoopPath, serialize(timelineLoop), 'utf8');

const ruleEvents = ruleEventsPayload();
const ruleEventsPath = join(here, 'recordings', 'rule-events.json');
writeFileSync(ruleEventsPath, serialize(ruleEvents), 'utf8');

const buffs = buffsPayload();
const buffsPath = join(here, 'recordings', 'buffs.json');
writeFileSync(buffsPath, serialize(buffs), 'utf8');

const decayFactor = decayFactorPayload();
const decayFactorPath = join(here, 'recordings', 'decay-factor.json');
writeFileSync(decayFactorPath, serialize(decayFactor), 'utf8');
console.log(`geschrieben: ${decayFactorPath}`);
console.log(
  `  Verwundbarkeit  Spur: ${decayFactor.factorAt.map((f) => f.vulnerability).join(' -> ')}`,
);

const decayTalent = decayTalentPayload();
const decayTalentPath = join(here, 'recordings', 'decay-talent.json');
writeFileSync(decayTalentPath, serialize(decayTalent), 'utf8');
console.log(`geschrieben: ${decayTalentPath}`);
console.log(
  `  Mit Talent      Spur: ${decayTalent.factorAt.map((f) => f.vulnerability).join(' -> ')}`,
);

const spenderTalent = spenderTalentPayload();
const spenderTalentPath = join(here, 'recordings', 'spender-talent.json');
writeFileSync(spenderTalentPath, serialize(spenderTalent), 'utf8');
console.log(`geschrieben: ${spenderTalentPath}`);
console.log(`  Entlader-Weg    Schaden: ${spenderTalent.expected.damage}`);

// Der Kontroll-Preis unter einer Verteilung aus zwei Baeumen (#419). Die erste
// Talent-Aufzeichnung mit MEHR als einem Weg — siehe `payloads.ts`, warum.
const kontrollPreisTalent = kontrollPreisTalentPayload();
const kontrollPreisTalentPath = join(here, 'recordings', `${KONTROLL_PREIS_TALENT_NAME}.json`);
writeFileSync(kontrollPreisTalentPath, serialize(kontrollPreisTalent), 'utf8');
console.log(`geschrieben: ${kontrollPreisTalentPath}`);
console.log(
  `  Kontroll-Preis  ${kontrollPreisTalent.expected.controlFailed} fehlgeschlagen, ` +
    `${kontrollPreisTalent.expected.controlSpent} Punkte ausgegeben, ` +
    `${kontrollPreisTalent.expected.latchCasts} LATCH`,
);

// Die zwei Schluesseltalente mit Zustand (#419, #418 AC4) — je eine Runde, die
// jeden Zweig ihres Zustands faehrt. Die Felder gehen nur mit Talent in die
// Pruefsumme ein; nur diese beiden Dateien tragen sie.
const schluesselNachlauf = schluesselNachlaufPayload();
const schluesselNachlaufPath = join(here, 'recordings', `${SCHLUESSEL_NACHLAUF_NAME}.json`);
writeFileSync(schluesselNachlaufPath, serialize(schluesselNachlauf), 'utf8');
console.log(`geschrieben: ${schluesselNachlaufPath}`);
console.log(
  `  Nachlauf        endet in Takt ${schluesselNachlauf.expected.echoEnds.join(' und ')}, ` +
    `${schluesselNachlauf.expected.decayStacks} Stapel am Ende`,
);
const schluesselNachlage = schluesselNachlagePayload();
const schluesselNachlagePath = join(here, 'recordings', `${SCHLUESSEL_NACHLAGE_NAME}.json`);
writeFileSync(schluesselNachlagePath, serialize(schluesselNachlage), 'utf8');
console.log(`geschrieben: ${schluesselNachlagePath}`);
console.log(
  `  Nachlage        in Takt ${schluesselNachlage.expected.refunds.join(' und ')}, ` +
    `Schild am Ende ${schluesselNachlage.expected.shield}`,
);
// Der gelegte Schild laeuft ungebrochen ab, danach brechen Umwandlungs-
// Schichten — und keine legt nach (Abnahme von #1146).
const schluesselNachlageAblauf = schluesselNachlageAblaufPayload();
const schluesselNachlageAblaufPath = join(
  here,
  'recordings',
  `${SCHLUESSEL_NACHLAGE_ABLAUF_NAME}.json`,
);
writeFileSync(schluesselNachlageAblaufPath, serialize(schluesselNachlageAblauf), 'utf8');
console.log(`geschrieben: ${schluesselNachlageAblaufPath}`);
console.log(
  `  Ablauf          Brueche in Takt ${schluesselNachlageAblauf.expected.breaks.join(' und ')}, ` +
    `Nachlagen: ${schluesselNachlageAblauf.expected.refunds.length}`,
);

const buffCombat = buffCombatPayload();
const buffCombatPath = join(here, 'recordings', 'buff-combat.json');
writeFileSync(buffCombatPath, serialize(buffCombat), 'utf8');
console.log(`geschrieben: ${buffCombatPath}`);
console.log(
  `  Verstaerkung im Kampf  ${buffCombat.expected.damage} Schaden, aktiv: ` +
    `${buffCombat.expected.buffWasActive}`,
);

const fixationSpells = fixationSpellsPayload();
const fixationSpellsPath = join(here, 'recordings', 'fixation-spells.json');
writeFileSync(fixationSpellsPath, serialize(fixationSpells), 'utf8');
console.log(`geschrieben: ${fixationSpellsPath}`);
console.log(
  `  Fixierung neu  ${fixationSpells.expected.damage} Schaden, ` +
    `${fixationSpells.expected.controlSpent} Punkte ausgegeben`,
);

const logKinds = logKindsPayload();
const logKindsPath = join(here, 'recordings', 'log-kinds.json');
writeFileSync(logKindsPath, serialize(logKinds), 'utf8');

const cooldownReset = cooldownResetPayload();
const cooldownResetPath = join(here, 'recordings', 'cooldown-reset.json');
writeFileSync(cooldownResetPath, serialize(cooldownReset), 'utf8');

const gear = gearPayload();
const gearPath = join(here, 'recordings', 'gear.json');
writeFileSync(gearPath, serialize(gear), 'utf8');

const sets = setsPayload();
const setsPath = join(here, 'recordings', 'sets.json');
writeFileSync(setsPath, serialize(sets), 'utf8');

const haste = hastePayload();
const hastePath = join(here, 'recordings', 'haste.json');
writeFileSync(hastePath, serialize(haste), 'utf8');

const worn = wornPayload();
const wornPath = join(here, 'recordings', `${WORN_NAME}.json`);
writeFileSync(wornPath, serialize(worn), 'utf8');

const sources = sourcesPayload();
const sourcesPath = join(here, 'recordings', 'sources.json');
writeFileSync(sourcesPath, serialize(sources), 'utf8');

const expected = recording['expected'];
const checksums = recording['checksums'];
const presses = recording['presses'];

console.log(`geschrieben: ${recordingPath}`);
console.log(
  `  Tastendrücke   ${presses.length} (angenommen ${expected['accepted']}, abgelehnt ${expected['rejected']})`,
);
console.log(`  Schaden        ${expected['damage']}`);
console.log(`  Sieg bei       ${expected['killedAtMs']} ms`);
console.log(`  Prüfsummen     ${checksums.length} Takte`);
console.log(`  Folge-Summe    ${expected['sequenceChecksum']}`);
console.log(`  Ziehungen      ${expected['randomDraws']}`);
console.log(`geschrieben: ${primitivesPath}`);

const m = misplays['expected'];
console.log(`geschrieben: ${misplaysPath}`);
console.log(
  `  Tastendrücke   ${misplays['presses'].length} (angenommen ${m['accepted']}, abgelehnt ${m['rejected']})`,
);
console.log(`  Ablehnungen    ${JSON.stringify(m['rejectedByReason'])}`);
console.log(`  Schaden        ${m['damage']}`);
console.log(`  verlorene DoT-Ticks ${m['lostDotTicks']}`);
console.log(`  Leerlauf-Takte ${m['idleTicks']}`);
console.log(`  Prüfsummen     ${misplays['checksums'].length} Takte`);
const b = bossState['expected'];
console.log(`geschrieben: ${bossStatePath}`);
console.log(`  Boss           ${bossState['bossId']}`);
console.log(`  Schaden        ${b['damage']}, Sieg bei ${b['killedAtMs']} ms`);
console.log(
  `  Endphase       ${b['finalPhase']}, Energie-Ausloesungen ${b['energyTriggers']}, Zeitleiste ${b['finalTimelineIndex']}`,
);
console.log(`  Pruefsummen    ${bossState['checksums'].length} Takte`);
console.log(`geschrieben: ${playerDamagePath}`);
for (const lauf of playerDamage['runs']) {
  const e = lauf['expected'];
  console.log(
    `  ${lauf['withInterrupt'] ? 'mit ' : 'ohne'} QUENCH   ` +
      `HP ${e['playerHp']}, erlitten ${e['damageTaken']}, ` +
      `unterbrochen ${e['interruptsLanded']}, durchgekommen ${e['interruptsMissed']}, ` +
      `Tod bei ${e['diedAtMs']} ms`,
  );
}
console.log(`geschrieben: ${coagulationPath}`);
for (const lauf of coagulation['runs']) {
  const e = lauf['expected'];
  console.log(
    `  ${String(lauf['way']).padEnd(7)} HP ${e['playerHp']}, erlitten ${e['damageTaken']}, ` +
      `abgefangen ${e['shieldAbsorbed']}, geheilt ${e['healed']}, ` +
      `umgewandelt ${e['converted']}, ausgeteilt ${e['damageDealt']}`,
  );
}
console.log(`geschrieben: ${fixationPath}`);
for (const lauf of fixation['runs']) {
  const e = lauf['expected'];
  const t = lauf['timelineIndexPerTick'];
  const rueckt = [];
  for (let i = 1; i < t.length; i++) if (t[i] !== t[i - 1]) rueckt.push(i * 10);
  console.log(
    `  Ausgabe ${String(lauf['withSpending']).padEnd(5)} ausgegeben ${e['controlSpent']}, ` +
      `Phase ${e['finalPhase']}, Schwelle -${e['phaseHoldPercent']}, ` +
      `Zeitleiste bei ${rueckt.join(', ')} ms`,
  );
}
console.log(`geschrieben: ${logParityPath}`);
console.log(`  je Art        ${JSON.stringify(logParity['counts'])}`);
console.log(`geschrieben: ${ruleEventsPath}`);
console.log(`geschrieben: ${buffsPath}`);
console.log(`geschrieben: ${logKindsPath}`);
console.log(`geschrieben: ${cooldownResetPath}`);
console.log(`  Auswahlmodus  ${cooldownReset['steps'].length} Schritte`);
console.log(`geschrieben: ${setsPath}`);
console.log(`  Saetze        ${sets['steps'].length} Zusammenstellungen`);
console.log(`geschrieben: ${gearPath}`);
console.log(
  `  Ausruestung   ${gear['poolSteps'].length} Topf-Schritte, ${gear['budgets'].length} Budgets, ${gear['slotNames'].length} Plaetze`,
);
console.log(
  `  Protokoll-Arten ${logKinds['kinds'].length}, davon ${logKinds['writtenByGame'].length} vom Spiel`,
);
console.log(`  Verstaerkungen ${buffs['steps'].length} Schritte, ${buffs['replacements']} ersetzt`);
console.log(
  `  Ereignisse     ${ruleEvents['emitted']}, Folge-Summe ${ruleEvents['sequenceChecksum']}`,
);
console.log(`geschrieben: ${rotPath}`);
console.log(
  `  ROT           ${rot['presses'].length} Druecke, ${rot['checksums'].length} Takte, ` +
    `Folge-Summe ${rot['expected']['sequenceChecksum']}`,
);
console.log(`geschrieben: ${newCalcinationPath}`);
console.log(`  neue Zauber   ${JSON.stringify(newCalcination['expected']['casts'])}`);

// Overdrive (#1177, Abschnitt 2): eine Zündung, zwei verworfene Drücke, beide
// Grenz-Takte des Fensters. Gegenstück: sim/tests/parity_overdrive.rs.
const overdrive = overdrivePayload();
const overdrivePath = join(here, 'recordings', `${OVERDRIVE_NAME}.json`);
writeFileSync(overdrivePath, serialize(overdrive), 'utf8');
console.log(`geschrieben: ${overdrivePath}`);
console.log(
  `  Overdrive     gezuendet bei ${overdrive['expected']['overdriveIgnitedAtMs']} ms, ` +
    `${overdrive['expected']['rejected']} verworfen, ${overdrive['checksums'].length} Takte`,
);
