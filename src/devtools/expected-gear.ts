/**
 * Der **erwartete Ausrüstungsstand** einer Instanz (GAME-DESIGN 20.5, #791).
 *
 * **Warum unter `devtools` und nicht bei der Ausrüstung.** Der erste Entwurf
 * lag in `src/gear/` — und ein Wächter hat ihn gefangen: von keinem
 * Einstiegspunkt erreichbar, also im ausgelieferten Spiel nicht vorhanden,
 * obwohl seine Tests grün waren. Das war kein Einbaufehler, sondern eine
 * falsche Einordnung. Der erwartete Stand ist ein **Maßstab zum Kalibrieren**,
 * kein Spielinhalt: Niemand im laufenden Spiel fragt danach, wie ein
 * Durchschnittsspieler ausgerüstet wäre.
 *
 * > „Ein Boss in Instanz 3 wird gegen die Ausrüstung gerechnet, die man in
 * > Instanz 2 findet — nicht gegen einen nackten Spieler und nicht gegen seine
 * > eigene Beute."
 *
 * # Die Annahme, in drei Teilen
 *
 * 20.5 verlangt ausdrücklich: „Sie wird gesetzt und benannt, nicht geraten."
 * Deshalb steht hier, woher jede der drei Größen kommt:
 *
 * | Größe            | Wert            | Herkunft                          |
 * | ---------------- | --------------- | --------------------------------- |
 * | Schwierigkeit    | normal (`d = 0`)| folgt aus 20.5 („beim ersten Betreten fordernd") |
 * | Qualität         | {@link weightsFor} | **gerechnet**, nicht gesetzt   |
 * | Zahl der Plätze  | alle zehn       | Entscheidung des Auftraggebers    |
 *
 * # Warum die Qualität gerechnet wird und nicht gewählt
 *
 * Der erste Vorschlag lautete „alle Plätze auf dem Deckel von
 * {@link qualityCapOf}". Für Instanz 2 hätte das 100 % `rare` bedeutet — die
 * Gewichte geben 15 % her. Eine **Überschätzung um Faktor sechs**, und jeder
 * daraus kalibrierte Boss wäre zu schwer geworden.
 *
 * Der Deckel sagt, was fallen **kann**; die Gewichte sagen, was fällt. Beide
 * werden gebraucht, und sie bedeuten Verschiedenes.
 *
 * # Warum alle zehn Plätze
 *
 * Die härtere Annahme, mit der Begründung des Auftraggebers: Ein zu leichter
 * Boss fällt beim Spielen auf und lässt sich nachziehen. Ein zu schwerer
 * vertreibt Spieler, bevor es jemand merkt.
 */

import { QUALITY_COMMON } from '../gear/budget.js';
import { itemLevelOf, qualityCapOf, weightsFor } from '../loot/rates.js';
import { SLOT_COUNT } from '../gear/slots.js';
import type { OwnedItemLike } from '../gear/catalog.js';

/**
 * Ein Stück je Platz, alle aus der `-mix`-Ausrichtung.
 *
 * **Die Ausrichtung ist eine Wahl und sie ist benannt.** `data/items.json`
 * führt zwei: `-mix` (catalysis, purity, yield) und `-drive` (essence, purity,
 * catalysis). Hier steht `-mix`, weil es der vollständige Satz über alle zehn
 * Plätze ist — `-drive` deckt nicht jeden ab.
 *
 * Das ist damit die **vierte** Annahme, kleiner als die drei oben, aber keine
 * Ableitung. Wer die Kalibrierung gegen `-drive` prüfen will, findet hier die
 * Stelle.
 */
const STUECK_JE_PLATZ: readonly string[] = [
  'focus-mix',
  'catalyst-mix',
  'sigil-mix',
  'relic-mix',
  'grimoire-mix',
  'shard-mix',
  'talisman-mix',
  'phylactery-mix',
  'anchor-mix',
  'prism-mix',
];

/** Die Schwierigkeit, auf der der erwartete Stand gerechnet wird (20.5). */
export const ERWARTETE_SCHWIERIGKEIT = 0;

/**
 * Wie viele der `plaetze` Stücke über `common` liegen — aus den Fallgewichten.
 *
 * Abgerundet: Bei 1500 von 10000 und zehn Plätzen sind das anderthalb Stücke,
 * also eines. **Die Rundung ist die vorsichtige Richtung** — sie unterschätzt
 * den Stand eher, und ein zu leichter Boss ist der weniger schädliche Fehler.
 */
export function bessereStuecke(instanz: number, plaetze: number): number {
  const gewichte = new Int32Array(4);
  weightsFor(instanz, ERWARTETE_SCHWIERIGKEIT, 0, gewichte);
  const gesamt = gewichte.reduce((a, b) => a + b, 0);
  if (gesamt <= 0) return 0;
  const anteilOberhalb = (gesamt - (gewichte[QUALITY_COMMON] ?? 0)) / gesamt;
  return Math.floor(plaetze * anteilOberhalb);
}

/**
 * Die Ausrüstung, mit der ein Spieler die Instanz **nach** `vorstufe` betritt.
 *
 * `plaetze` ist absichtlich ein Parameter und kein fester Wert: Die
 * Entscheidung lautet zwar „alle zehn", aber die Schranken zu rechnen
 * (drei gegen zehn) hat den Unterschied überhaupt erst sichtbar gemacht. Ein
 * fester Wert nähme dem nächsten Leser dieses Werkzeug weg.
 */
export function erwarteterStand(
  vorstufe: number,
  plaetze: number = SLOT_COUNT,
): readonly OwnedItemLike[] {
  const itemLevel = itemLevelOf(vorstufe, ERWARTETE_SCHWIERIGKEIT, 0);
  const bessere = bessereStuecke(vorstufe, plaetze);
  const raus: OwnedItemLike[] = [];
  for (let platz = 0; platz < plaetze; platz++) {
    const itemId = STUECK_JE_PLATZ[platz];
    if (itemId === undefined) continue;
    raus.push({
      itemId,
      upgradeSteps: 0,
      equippedSlot: platz,
      itemLevel,
      quality: platz < bessere ? qualityCapOf(vorstufe) : QUALITY_COMMON,
    });
  }
  return raus;
}
