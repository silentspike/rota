/**
 * Die Annahme hinter der Kalibrierung, festgenagelt (GAME-DESIGN 20.5, #791).
 *
 * **Warum als Prüffall und nicht als Absatz im Design.** 20.5 verlangt, dass
 * die Annahme „gesetzt und benannt" wird. Benannt ist sie dort; hier steht,
 * dass der Code sie auch einhält. Ein Satz im Dokument altert schweigend — an
 * genau dieser Messreihe ist das an einem Tag achtmal passiert.
 *
 * Geprüft wird die **Herkunft** jeder der drei Größen, nicht ihr Wert allein:
 * Eine Zahl, die zufällig stimmt, ist hier wertlos.
 */

import { describe, expect, it } from 'vitest';

import {
  ERWARTETE_SCHWIERIGKEIT,
  bessereStuecke,
  erwarteterStand,
} from '../../src/devtools/expected-gear.js';
import { QUALITY_COMMON, QUALITY_RARE } from '../../src/gear/budget.js';
import { itemLevelOf } from '../../src/loot/rates.js';
import { SLOT_COUNT } from '../../src/gear/slots.js';

describe('#791 — der erwartete Ausrüstungsstand', () => {
  /** Schwierigkeit normal, weil 20.5 „beim ersten Betreten fordernd" sagt. */
  it('rechnet auf normal', () => {
    expect(ERWARTETE_SCHWIERIGKEIT).toBe(0);
  });

  /** Ohne Angabe alle Plätze — die Entscheidung des Auftraggebers. */
  it('belegt ohne Angabe alle zehn Plätze', () => {
    expect(erwarteterStand(2).length).toBe(SLOT_COUNT);
  });

  /**
   * **Die Zahl, um die es ging.** Für Instanz 2 geben die Gewichte 1500 von
   * 10000 oberhalb von common her — bei zehn Plätzen also anderthalb Stücke,
   * abgerundet eines.
   *
   * Der zuerst vorgeschlagene Wert war „alle zehn auf dem Deckel", also zehn.
   * Steht hier je wieder 10, ist die Rechnung durch eine Wahl ersetzt worden.
   */
  it('leitet die Qualität aus den Fallgewichten ab, nicht aus dem Deckel', () => {
    expect(bessereStuecke(2, 10)).toBe(1);
  });

  /**
   * Die Gegenrichtung zum obigen Fall: Die Stücke oberhalb von common tragen
   * den Deckel, der Rest common. Wäre alles gleich, hinge die Zahl oben an
   * nichts.
   */
  it('mischt die Qualitäten, statt eine zu nehmen', () => {
    const qualitaeten = erwarteterStand(2, 10).map((s) => s.quality);
    expect(qualitaeten.filter((q) => q === QUALITY_RARE).length).toBe(1);
    expect(qualitaeten.filter((q) => q === QUALITY_COMMON).length).toBe(9);
  });

  /**
   * **Gegen die Vorstufe, nicht gegen die eigene Instanz.** Der Aufrufer
   * übergibt die Vorstufe, und die Stufe muss die ihre sein — hier für
   * Instanz 2 die 7, nicht die 13 von Instanz 3.
   */
  it('nimmt die Gegenstandsstufe der übergebenen Instanz', () => {
    expect(erwarteterStand(2)[0]?.itemLevel).toBe(itemLevelOf(2, 0, 0));
    expect(erwarteterStand(2)[0]?.itemLevel).toBe(7);
  });

  /** Jedes Stück liegt auf einem eigenen Platz — sonst zählt `wornFrom` es nicht. */
  it('legt jedes Stück auf einen eigenen Platz', () => {
    const plaetze = erwarteterStand(2).map((s) => s.equippedSlot);
    expect(new Set(plaetze).size).toBe(SLOT_COUNT);
    expect(plaetze).not.toContain(null);
  });
});
