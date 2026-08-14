/**
 * Wie viele UNTERSCHEIDBARE Ausrüstungs-Wirkungen gibt es? (Teil A)
 *
 * ```text
 * npx vite-node tools/phaenotypen-gear.mjs
 * ```
 *
 * # Die Frage
 *
 * Der Kampf sieht von einer Ausrüstung nur zweierlei: die Summe der Werte
 * (`WornValues`) und welche Satz-Effekte anliegen. Zwei verschiedene
 * Zusammenstellungen, die darin übereinstimmen, sind für ihn dasselbe.
 *
 *   Genotyp    welche Stücke getragen werden
 *   Phänotyp   was davon im Kampf ankommt
 *
 * Das Verhältnis sagt, wie stark sich ein Suchraum zusammenfalten lässt.
 *
 * # Die Einstellungen dieses Laufs
 *
 *   Quelle          data/items.json — 27 Stücke, alle `itemLevel` 40,
 *                   `quality` 2 (epic)
 *   Belegung        genau EIN Stück je Platz, alle zehn Plätze besetzt
 *   Genotypen       3^7 * 2^3 = 17496 — vollständig aufgezählt, keine Stichprobe
 *   Phänotyp        (WornValues, Satz-Effekt-Maske), verglichen als
 *                   verketteter Schlüssel
 *
 * **Gewürfelte Stufe und Qualität bleiben außen vor.** `OwnedItemLike` erlaubt
 * sie zu überschreiben; hier zählt die Vorlage, weil die Frage dem Katalog
 * gilt und nicht dem Beutezufall.
 *
 * # Was diese Zahl NICHT sagt
 *
 * Fünf der sechs Sätze aus `data/sets.json` haben **kein einziges** Stück im
 * Katalog — alle sechs Satz-Stücke gehören zu Satz 0 („Test Set"). Die
 * Satz-Effekt-Maske ist deshalb heute fast eindimensional, und der gemessene
 * Faltungsfaktor gilt für einen Katalog mit einem belegten Satz.
 */

import { readFileSync } from 'node:fs';

import { wornFrom, rotationEffectsFrom } from '../src/gear/catalog.js';
import { SLOT_COUNT } from '../src/gear/slots.js';

const items = JSON.parse(readFileSync('data/items.json', 'utf8'));
const alle = Array.isArray(items) ? items : (items.items ?? []);

/** Die Stücke je Platz — die Achsen des Genotyp-Raums. */
const jePlatz = [];
for (let platz = 0; platz < SLOT_COUNT; platz++) {
  jePlatz.push(alle.filter((x) => x.slot === platz).map((x) => x.id));
}

const genotypen = jePlatz.reduce((n, liste) => n * liste.length, 1);
console.log(`Stuecke je Platz: ${jePlatz.map((l) => l.length).join(' ')}`);
console.log(`Genotypen: ${genotypen}`);

/**
 * Der Schlüssel eines Phänotyps.
 *
 * **Alle Felder von `WornValues`, nicht nur die vier Kampfwerte.** Ein
 * Schlüssel, der `gear_effects` oder `vessel` wegließe, würde verschiedene
 * Wirkungen als gleich zählen und den Faktor zu groß erscheinen lassen.
 */
function schluessel(worn, effekte) {
  const werte = Object.keys(worn)
    .sort()
    .map((k) => `${k}=${String(worn[k])}`)
    .join(',');
  return `${werte}|${[...effekte].sort().join(',')}`;
}

const gesehen = new Set();
const zaehler = new Array(SLOT_COUNT).fill(0);
for (let n = 0; n < genotypen; n++) {
  const getragen = [];
  for (let platz = 0; platz < SLOT_COUNT; platz++) {
    getragen.push({
      itemId: jePlatz[platz][zaehler[platz]],
      upgradeSteps: 0,
      equippedSlot: platz,
    });
  }
  gesehen.add(schluessel(wornFrom(getragen), rotationEffectsFrom(getragen)));

  // Zähler weiterdrehen — die Stellen laufen wie ein Kilometerzähler.
  for (let platz = 0; platz < SLOT_COUNT; platz++) {
    zaehler[platz] += 1;
    if (zaehler[platz] < jePlatz[platz].length) break;
    zaehler[platz] = 0;
  }
}

const faktor = genotypen / gesehen.size;
console.log(`Phaenotypen: ${gesehen.size}`);
console.log(
  `FAKTOR: aus ${genotypen} Genotypen werden ${gesehen.size} Phaenotypen, Faktor ${faktor.toFixed(2)}`,
);

// **Träger-Zusicherung.** Eine Faltung von 312 wäre auch das Ergebnis eines
// Schlüssels, der zu wenig unterscheidet. Zwei Gegenproben:
//
//   1. Verschiedene Stücke auf demselben Platz müssen verschiedene Schlüssel
//      ergeben — sonst zählt die Messung Gleiches, wo Verschiedenes ist.
//   2. Dieselbe Zusammenstellung zweimal muss denselben Schlüssel ergeben.
const platz0 = jePlatz[0];
const s = (id) =>
  schluessel(
    wornFrom([{ itemId: id, upgradeSteps: 0, equippedSlot: 0 }]),
    rotationEffectsFrom([{ itemId: id, upgradeSteps: 0, equippedSlot: 0 }]),
  );
const verschieden = new Set(platz0.map(s)).size;
console.log(
  `Gegenprobe 1: ${platz0.length} Stuecke auf Platz 0 ergeben ${verschieden} verschiedene Schluessel`,
);
console.log(
  `Gegenprobe 2: derselbe Aufbau zweimal gleich? ${String(s(platz0[0]) === s(platz0[0]))}`,
);
