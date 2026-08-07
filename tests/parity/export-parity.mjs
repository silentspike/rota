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
  fixationSpellsPayload,
  resetRunPayload,
  raceHealPayload,
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
  cooldownResetPayload,
  misplaysPayload,
  synergyPayload,
  SYNERGY_NAME,
  synergyHastePayload,
  SYNERGY_HASTE_NAME,
  wardCapPayload,
  WARD_CAP_NAME,
  putrefactionFillerPayload,
  PUTREFACTION_FILLER_NAME,
  buffTargetsPayload,
  BUFF_TARGETS_NAME,
  cinderPayload,
  dotLifetimePayload,
  DOT_LIFETIME_NAME,
  extendEffectsPayload,
  EXTEND_EFFECTS_NAME,
  kernePayload,
  KERNE_NAME,
  CINDER_NAME,
  playerDamagePayload,
  coagulationPayload,
  fixationPayload,
  logParityPayload,
  timelineLoopPayload,
  sublimationPayload,
  SUBLIMATION_NAME,
  wispPayload,
  WISP_NAME,
  alembicPayload,
  ALEMBIC_NAME,
  TIMELINE_LOOP_NAME,
  primitivesPayload,
  recordingPayload,
  ruleEventsPayload,
  MISPLAY_NAME,
  RECORDING_NAME,
  VESSEL_NAME,
  DOOM_STACKS_NAME,
  RESET_RUN_NAME,
  RACE_HEAL_NAME,
  PLAYER_DAMAGE_NAME,
  COAGULATION_NAME,
  FIXATION_NAME,
  LOG_PARITY_NAME,
  serialize,
  RUNAWAY_PROGRESS_NAME,
  runawayProgressPayload,
  LEVEL_30_NAME,
  level30Payload,
  CAST_TIME_NAME,
  castTimePayload,
  RESISTANCE_NAME,
  resistancePayload,
  RESISTANCE_MIXED_NAME,
  resistanceMixedPayload,
  newCalcinationPayload,
  NEW_CALCINATION_NAME,
} from './payloads.ts';

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

const extendEffects = extendEffectsPayload();
const extendEffectsPath = join(here, 'recordings', `${EXTEND_EFFECTS_NAME}.json`);
writeFileSync(extendEffectsPath, serialize(extendEffects), 'utf8');

// Die fuenf Verstaerkungen aus #540. Ohne sie wirkt KEINE Aufzeichnung
// flashpoint, quicksilver, distil, griplock oder virulence — ihre
// Bitgleichheit waere eine Annahme.
const buffTargets = buffTargetsPayload();
const buffTargetsPath = join(here, 'recordings', `${BUFF_TARGETS_NAME}.json`);
writeFileSync(buffTargetsPath, serialize(buffTargets), 'utf8');

const kerne = kernePayload();
const kernePath = join(here, 'recordings', `${KERNE_NAME}.json`);
writeFileSync(kernePath, `${JSON.stringify(kerne, null, 2)}\n`);
console.log(`geschrieben: ${kernePath}`);
console.log(`  gewirkt   ${JSON.stringify(kerne.expected.casts)}`);

const bossState = bossStatePayload();
const bossStatePath = join(here, 'recordings', `${VESSEL_NAME}.json`);
writeFileSync(bossStatePath, serialize(bossState), 'utf8');

const doomStacks = doomStacksPayload();
const doomStacksPath = join(here, 'recordings', `${DOOM_STACKS_NAME}.json`);
writeFileSync(doomStacksPath, serialize(doomStacks), 'utf8');

const resetRun = resetRunPayload();
const resetRunPath = join(here, 'recordings', `${RESET_RUN_NAME}.json`);
writeFileSync(resetRunPath, serialize(resetRun), 'utf8');

const raceHeal = raceHealPayload();
const raceHealPath = join(here, 'recordings', `${RACE_HEAL_NAME}.json`);
writeFileSync(raceHealPath, serialize(raceHeal), 'utf8');

const trialBlocks = trialBlocksPayload();
const trialBlocksPath = join(here, 'recordings', `${TRIAL_BLOCKS_NAME}.json`);
writeFileSync(trialBlocksPath, serialize(trialBlocks), 'utf8');

const level30 = level30Payload();
const level30Path = join(here, 'recordings', `${LEVEL_30_NAME}.json`);
writeFileSync(level30Path, serialize(level30), 'utf8');
console.log(`geschrieben: ${level30Path}`);
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

const fixation = fixationPayload();
const fixationPath = join(here, 'recordings', `${FIXATION_NAME}.json`);
writeFileSync(fixationPath, serialize(fixation), 'utf8');

const logParity = logParityPayload();
const logParityPath = join(here, 'recordings', `${LOG_PARITY_NAME}.json`);
writeFileSync(logParityPath, serialize(logParity), 'utf8');

const sublimation = sublimationPayload();
const sublimationPath = join(here, 'recordings', `${SUBLIMATION_NAME}.json`);
writeFileSync(sublimationPath, serialize(sublimation), 'utf8');

const alembic = alembicPayload();
const alembicPath = join(here, 'recordings', `${ALEMBIC_NAME}.json`);
writeFileSync(alembicPath, serialize(alembic), 'utf8');
console.log(`geschrieben: ${alembicPath}`);
console.log(
  `  ALEMBIC   ${alembic.expected.alembicCasts} Wurf, ` +
    `${alembic.expected.auswahlen} greifende Auswahlen`,
);

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
console.log(`geschrieben: ${newCalcinationPath}`);
console.log(`  neue Zauber   ${JSON.stringify(newCalcination['expected']['casts'])}`);
