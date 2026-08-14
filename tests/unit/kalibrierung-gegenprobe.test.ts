/**
 * Rechnet die Kalibrierung nach 20.5 anders als die heutige? (#791, Punkt 3)
 *
 * **Die Frage ist nicht rhetorisch.** GAME-DESIGN 20.5 verlangt, Bosse gegen
 * den erwarteten Ausrüstungsstand ihrer Instanz zu rechnen statt gegen einen
 * nackten Spieler. Ob das überhaupt einen Unterschied macht — und wie groß er
 * ist — steht hier mit Zahlen, nicht als Ableitung.
 *
 * # Warum hier trotzdem eine Spanne steht
 *
 * Der erwartete Stand hat drei Stellschrauben, und alle drei sind inzwischen
 * geklärt (20.5): Schwierigkeit **normal**, Qualität aus `weightsFor`, **alle
 * zehn Plätze**. Der gesetzte Stand ist damit der Fall `plaetze = 10`.
 *
 * Die drei Plätze bleiben als **unterer** Fall daneben stehen, und zwar
 * absichtlich: Solange nur der obere gemessen ist, sagt eine Zahl über dem
 * nackten Maßstab wenig — sie könnte allein an der Menge hängen. Erst dass
 * **auch** der dünnste Stand darüber liegt, macht die Aussage „die heutige
 * Kalibrierung ist systematisch zu niedrig" belastbar.
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
import { erwarteterStand } from '../../src/devtools/expected-gear.js';

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
