/**
 * Schreibt die gemeinsamen Vergleichsdateien für den Abgleich mit dem Simulator.
 *
 * ```text
 * npx vite-node tests/parity/export-parity.ts
 * ```
 *
 * Fünf Dateien entstehen dabei, beide unter `tests/parity/recordings/`:
 *
 * | Datei | Inhalt |
 * | --- | --- |
 * | `cooldown-precision.json` | Eine aufgezeichnete Runde samt Ergebnis und Prüfsumme je Takt |
 * | `primitives.json` | Zufallsströme und Prüfsumme — die Bausteine darunter |
 * | `rule-events.json` | Eine feste Folge von Regel-Ereignissen samt Prüfsumme |
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
  cooldownResetPayload,
  gearPayload,
  logKindsPayload,
  bossStatePayload,
  misplaysPayload,
  playerDamagePayload,
  coagulationPayload,
  fixationPayload,
  primitivesPayload,
  recordingPayload,
  ruleEventsPayload,
  MISPLAY_NAME,
  RECORDING_NAME,
  VESSEL_NAME,
  PLAYER_DAMAGE_NAME,
  COAGULATION_NAME,
  FIXATION_NAME,
  serialize,
} from './payloads.ts';

const here = dirname(fileURLToPath(import.meta.url));

const recording = recordingPayload();
const recordingPath = join(here, 'recordings', `${RECORDING_NAME}.json`);
writeFileSync(recordingPath, serialize(recording), 'utf8');

const primitives = primitivesPayload();
const primitivesPath = join(here, 'recordings', 'primitives.json');
writeFileSync(primitivesPath, serialize(primitives), 'utf8');

const misplays = misplaysPayload();
const misplaysPath = join(here, 'recordings', `${MISPLAY_NAME}.json`);
writeFileSync(misplaysPath, serialize(misplays), 'utf8');

const bossState = bossStatePayload();
const bossStatePath = join(here, 'recordings', `${VESSEL_NAME}.json`);
writeFileSync(bossStatePath, serialize(bossState), 'utf8');

const playerDamage = playerDamagePayload();
const playerDamagePath = join(here, 'recordings', `${PLAYER_DAMAGE_NAME}.json`);
writeFileSync(playerDamagePath, serialize(playerDamage), 'utf8');

const coagulation = coagulationPayload();
const coagulationPath = join(here, 'recordings', `${COAGULATION_NAME}.json`);
writeFileSync(coagulationPath, serialize(coagulation), 'utf8');

const fixation = fixationPayload();
const fixationPath = join(here, 'recordings', `${FIXATION_NAME}.json`);
writeFileSync(fixationPath, serialize(fixation), 'utf8');

const ruleEvents = ruleEventsPayload();
const ruleEventsPath = join(here, 'recordings', 'rule-events.json');
writeFileSync(ruleEventsPath, serialize(ruleEvents), 'utf8');

const buffs = buffsPayload();
const buffsPath = join(here, 'recordings', 'buffs.json');
writeFileSync(buffsPath, serialize(buffs), 'utf8');

const logKinds = logKindsPayload();
const logKindsPath = join(here, 'recordings', 'log-kinds.json');
writeFileSync(logKindsPath, serialize(logKinds), 'utf8');

const cooldownReset = cooldownResetPayload();
const cooldownResetPath = join(here, 'recordings', 'cooldown-reset.json');
writeFileSync(cooldownResetPath, serialize(cooldownReset), 'utf8');

const gear = gearPayload();
const gearPath = join(here, 'recordings', 'gear.json');
writeFileSync(gearPath, serialize(gear), 'utf8');

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
console.log(`geschrieben: ${ruleEventsPath}`);
console.log(`geschrieben: ${buffsPath}`);
console.log(`geschrieben: ${logKindsPath}`);
console.log(`geschrieben: ${cooldownResetPath}`);
console.log(`  Auswahlmodus  ${cooldownReset['steps'].length} Schritte`);
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
