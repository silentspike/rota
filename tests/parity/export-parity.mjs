/**
 * Schreibt die gemeinsamen Vergleichsdateien für den Abgleich mit dem Simulator.
 *
 * ```text
 * npx vite-node tests/parity/export-parity.ts
 * ```
 *
 * Zwei Dateien entstehen dabei, beide unter `tests/parity/recordings/`:
 *
 * | Datei | Inhalt |
 * | --- | --- |
 * | `cooldown-precision.json` | Eine aufgezeichnete Runde samt Ergebnis und Prüfsumme je Takt |
 * | `primitives.json` | Zufallsströme und Prüfsumme — die Bausteine darunter |
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

import { primitivesPayload, recordingPayload, RECORDING_NAME, serialize } from './payloads.ts';

const here = dirname(fileURLToPath(import.meta.url));

const recording = recordingPayload();
const recordingPath = join(here, 'recordings', `${RECORDING_NAME}.json`);
writeFileSync(recordingPath, serialize(recording), 'utf8');

const primitives = primitivesPayload();
const primitivesPath = join(here, 'recordings', 'primitives.json');
writeFileSync(primitivesPath, serialize(primitives), 'utf8');

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
