/**
 * Wie viele Phänotypen ergäben SECHS belegte Sätze? (Hochrechnung)
 *
 * ```text
 * npx vite-node tools/phaenotypen-gear-hochrechnung.mjs
 * ```
 *
 * **Dies ist keine Messung des Bestands, sondern eine Hochrechnung.** Heute
 * hat nur Satz 0 („Test Set") Gegenstände; die Sätze 1 bis 5 aus
 * `data/sets.json` haben null. Der gemessene Bestand steht in
 * `phaenotypen-gear.mjs` (17496 → 56, Faktor 312).
 *
 * # Die Annahmen, und sie sind als solche gekennzeichnet
 *
 * 1. **Sechs Sätze mit je sechs Stücken** — vom Orchestrator aus dem Plan
 *    übernommen: „einer je Endgame-Instanz" (`sets.rs:37`), sechs
 *    Endgame-Instanzen. Die drei Stufen (2/4/6 Teile) verlangen mindestens
 *    sechs Plätze, damit die dritte erreichbar ist.
 *
 * 2. **Jeder Satz belegt die Plätze 1 bis 6** — meine Annahme, nicht aus dem
 *    Plan. Er sagt nicht, welche Plätze ein Satz belegt; ich nehme das
 *    heutige Muster (`test-set-1` bis `-6` liegen auf 1 bis 6).
 *
 * 3. **21 satzlose Stücke, nicht 27.** Gemessen: Von den 27 Stücken tragen
 *    sechs bereits ein `setId`. Die Vorgabe lautete „36 neben den 27
 *    satzlosen" — die 27 ist aber die Gesamtzahl. Mit 27 wären es 63 Stücke
 *    statt 57; die Zahl steht unten für beide Fassungen.
 *
 * # Der Genotyp-Raum
 *
 * ```text
 *   Platz 0        3 Stücke   (nur satzlos)
 *   Plätze 1-6     2 + 6 = 8  (satzlos plus einer je Satz)
 *   Plätze 7-9     2 Stücke   (nur satzlos)
 *                            3 * 8^6 * 2^3 = 6 291 456
 * ```
 *
 * Vollständig aufgezählt, keine Stichprobe.
 */

import { readFileSync } from 'node:fs';

import { wornFrom, rotationEffectsFrom } from '../src/gear/catalog.js';
import { SLOT_COUNT } from '../src/gear/slots.js';

const roh = JSON.parse(readFileSync('data/items.json', 'utf8'));
const alle = Array.isArray(roh) ? roh : (roh.items ?? []);

/** Die satzlosen Stücke je Platz — der gemessene Bestand. */
const satzlos = [];
for (let platz = 0; platz < SLOT_COUNT; platz++) {
  satzlos.push(alle.filter((x) => x.slot === platz && x.setId === undefined).map((x) => x.id));
}

/**
 * Die erfundenen Satz-Stücke: sechs Sätze, je Platz 1 bis 6.
 *
 * Sie stehen NICHT im Katalog — deshalb kann `wornFrom` sie nicht auflösen.
 * Der Phänotyp wird hier aus denselben Bestandteilen gebildet, die
 * `wornFrom` benutzt: Werte aus Stufe und Qualität, plus die Satz-Kennung.
 */
const SAETZE = 6;
const SATZ_PLAETZE = [1, 2, 3, 4, 5, 6];

const jePlatz = [];
for (let platz = 0; platz < SLOT_COUNT; platz++) {
  const liste = satzlos[platz].map((id) => ({ id, setId: null }));
  if (SATZ_PLAETZE.includes(platz)) {
    for (let s = 0; s < SAETZE; s++) liste.push({ id: `satz-${s}-platz-${platz}`, setId: s });
  }
  jePlatz.push(liste);
}

const genotypen = jePlatz.reduce((n, l) => n * l.length, 1);
console.log(`Stuecke je Platz: ${jePlatz.map((l) => l.length).join(' ')}`);
console.log(`Genotypen: ${genotypen}`);

/**
 * Der Phänotyp einer Zusammenstellung.
 *
 * Die satzlosen Stücke gehen über `wornFrom` ein — echte Werte aus dem
 * Katalog. Die erfundenen tragen dieselben Werte wie ein heutiges Satz-Stück
 * (`test-set-1`), weil alle 27 Stücke `itemLevel` 40 und `quality` 2 haben;
 * unterschieden werden sie nur durch ihre Satz-Kennung.
 *
 * **Das ist die Stelle, an der die Hochrechnung ungenau wird**, und sie ist
 * hier benannt: Ein echter Katalog gäbe den neuen Sätzen eigene Affix-Muster.
 * Diese Rechnung unterstellt, dass sie sich nur im Satz unterscheiden — sie
 * liefert damit eine UNTERGRENZE der Phänotypen.
 */
function phaenotyp(auswahl) {
  const echte = auswahl.filter((x) => x.setId === null);
  const getragen = echte.map((x, i) => ({
    itemId: x.id,
    upgradeSteps: 0,
    equippedSlot: jePlatz.findIndex((l) => l.some((y) => y.id === x.id)),
  }));
  const worn = wornFrom(getragen);
  const werte = Object.keys(worn)
    .sort()
    .map((k) => `${k}=${String(worn[k])}`)
    .join(',');
  const jeSatz = new Array(SAETZE).fill(0);
  for (const x of auswahl) if (x.setId !== null) jeSatz[x.setId] += 1;
  return `${werte}|${jeSatz.join('.')}|${rotationEffectsFrom(getragen).sort().join(',')}`;
}

const gesehen = new Set();
const zaehler = new Array(SLOT_COUNT).fill(0);
const start = process.hrtime.bigint();
for (let n = 0; n < genotypen; n++) {
  const auswahl = [];
  for (let platz = 0; platz < SLOT_COUNT; platz++) auswahl.push(jePlatz[platz][zaehler[platz]]);
  gesehen.add(phaenotyp(auswahl));
  for (let platz = 0; platz < SLOT_COUNT; platz++) {
    zaehler[platz] += 1;
    if (zaehler[platz] < jePlatz[platz].length) break;
    zaehler[platz] = 0;
  }
}
const dauerMs = Number((process.hrtime.bigint() - start) / 1000000n);

console.log(`Phaenotypen: ${gesehen.size}`);
console.log(
  `FAKTOR: aus ${genotypen} Genotypen werden ${gesehen.size} Phaenotypen, Faktor ${(genotypen / gesehen.size).toFixed(2)}`,
);
console.log(`Dauer: ${dauerMs} ms`);
