/**
 * Rechnet die Kalibrierung nach 20.5 anders als die heutige? (#791, Punkt 3)
 *
 * **Die Frage ist nicht rhetorisch.** GAME-DESIGN 20.5 verlangt, Bosse gegen
 * den erwarteten Ausrüstungsstand ihrer Instanz zu rechnen statt gegen einen
 * nackten Spieler. Ob das überhaupt einen Unterschied macht — und wie groß er
 * ist — steht hier mit Zahlen, nicht als Ableitung.
 *
 * # Warum eine Spanne und keine Zahl
 *
 * Der erwartete Stand hat drei Stellschrauben (Meldung 7463): Schwierigkeit,
 * Zahl der belegten Plätze, Qualität. Zwei davon sind geklärt — Schwierigkeit
 * **normal** (20.5: „beim ersten Betreten fordernd") und die Qualität aus
 * `weightsFor`. Die dritte, **wie viele der zehn Plätze belegt sind**, liegt
 * beim Auftraggeber.
 *
 * Diese Datei rechnet deshalb die **Schranken** statt einer Zahl: drei Plätze
 * als unterer, zehn als oberer Fall. Wie der Auftraggeber auch entscheidet —
 * die Antwort liegt dazwischen und ist schon gemessen.
 *
 * # Was hier NICHT geprüft wird
 *
 * Ob die Bosse richtig kalibriert **sind**. Diese Datei zeigt nur, dass die
 * beiden Maßstäbe auseinanderliegen. Bossdaten anzufassen ist ausdrücklich
 * nicht Teil dieser Arbeit.
 */

import { describe, expect, it } from 'vitest';

import { wornFrom, type OwnedItemLike } from '../../src/gear/catalog.js';
import { runReference } from '../../src/devtools/reference-run.js';
import { itemLevelOf, qualityCapOf, weightsFor } from '../../src/loot/rates.js';
import { QUALITY_COMMON } from '../../src/gear/budget.js';

/** Ein Stück je Platz, alle aus der `-mix`-Ausrichtung (catalysis, purity, yield). */
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

/**
 * Der erwartete Stand eines Spielers, der Instanz `vorstufe` auf normal
 * gespielt hat und `plaetze` Stücke daraus trägt.
 *
 * **Die Qualität wird gerechnet, nicht gewählt.** `weightsFor` liefert die
 * Verteilung, und der Anteil oberhalb von common bestimmt, wie viele Stücke
 * die höhere Qualität tragen. Für Instanz 2 sind das 1500 von 10000 — bei
 * zehn Plätzen also anderthalb Stück, abgerundet eines.
 */
function erwarteterStand(vorstufe: number, plaetze: number): readonly OwnedItemLike[] {
  const itemLevel = itemLevelOf(vorstufe, 0, 0);
  const gewichte = new Int32Array(4);
  weightsFor(vorstufe, 0, 0, gewichte);
  const gesamt = gewichte.reduce((a, b) => a + b, 0);
  const anteilOberhalb = gesamt > 0 ? (gesamt - (gewichte[QUALITY_COMMON] ?? 0)) / gesamt : 0;
  const bessere = Math.floor(plaetze * anteilOberhalb);

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

function durchsatz(items: readonly OwnedItemLike[]): number {
  return runReference('good', { stopOnKill: false, worn: wornFrom(items) }).damage;
}

describe('#791 Punkt 3 — die neue Kalibrierung rechnet anders', () => {
  /**
   * **Träger-Zusicherung, zweiteilig.** Alles Weitere vergleicht Schaden mit
   * und ohne Ausrüstung. Trägt der erwartete Stand gar keine Werte, wären alle
   * Vergleiche unten erfüllt und nichtssagend — genau der Fall, der in dieser
   * Messreihe schon einmal vier Nullen erzeugt hat.
   */
  it('der erwartete Stand trägt überhaupt Werte', () => {
    const worn = wornFrom(erwarteterStand(2, 10));
    expect(worn.catalysis + worn.purity + worn.yieldBonus + worn.essence).toBeGreaterThan(0);
  });

  /**
   * Der Kern von Punkt 3: Der nackte Maßstab und der erwartete liegen
   * auseinander. Wäre das nicht so, wäre 20.5 eine Umbenennung.
   */
  it('erwarteter Stand liefert mehr Durchsatz als nackt', () => {
    const nackt = runReference('good', { stopOnKill: false }).damage;
    expect(durchsatz(erwarteterStand(2, 10))).toBeGreaterThan(nackt);
  });

  /**
   * Die Spanne, in der die offene Frage (b) liegt: Drei Plätze sind der
   * untere Fall, zehn der obere. **Beide liegen über dem nackten Maßstab** —
   * wie der Auftraggeber auch entscheidet, die heutige Kalibrierung ist zu
   * niedrig.
   */
  it('auch der untere Fall liegt über dem nackten Maßstab', () => {
    const nackt = runReference('good', { stopOnKill: false }).damage;
    const wenig = durchsatz(erwarteterStand(2, 3));
    const viel = durchsatz(erwarteterStand(2, 10));
    expect(wenig).toBeGreaterThan(nackt);
    expect(viel).toBeGreaterThan(wenig);
  });

  /**
   * Eine spätere Instanz muss einen höheren Maßstab ergeben als eine frühere —
   * sonst trüge die Kalibrierung den Fortschritt nicht.
   *
   * **Das ist die Prüfung, die eine vertauschte Vorstufe fängt.** Wer
   * `itemLevelOf(instanz)` statt `itemLevelOf(instanz - 1)` einsetzt, bekommt
   * weiterhin steigende Zahlen — aber diese Zeile hält fest, gegen WELCHE
   * Instanz gerechnet wird.
   */
  it('eine spätere Vorstufe ergibt einen höheren Maßstab', () => {
    expect(durchsatz(erwarteterStand(8, 10))).toBeGreaterThan(durchsatz(erwarteterStand(2, 10)));
  });
});
