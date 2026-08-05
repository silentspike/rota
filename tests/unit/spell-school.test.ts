/**
 * Jeder Zauber weiss, aus welcher Schule er kommt (Issue #270, AC0).
 *
 * WARUM DAS FELD NOETIG WAR
 * -------------------------
 * `SchoolDef` traegt eine Schule, und es gibt genau eine: Alle fuenfzehn
 * Zauber haengen an ihr, weil sie beim Zusammenlegen aus sechs Dateien in EINE
 * Liste wandern. Die Herkunft ging dabei verloren — gemessen am 2026-08-05:
 * 0 von 15 Zaubern trugen ein `school`-Feld.
 *
 * Ohne sie ist „nach Schule gruppiert" (GAME-DESIGN 16) nicht schwer, sondern
 * unmoeglich.
 *
 * WAS SICH NICHT AENDERN DARF
 * ---------------------------
 * Die Reihenfolge. Sie bestimmt die Indizes, und die stehen in jeder
 * aufgezeichneten Runde (GAME-DESIGN 18.3) — sie umzustellen deutete jede
 * alte Aufzeichnung still um. Der erste Fall unten haelt sie fest.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';

const SPELLS = CALCINATION.value.spells;

describe('AC0 — die Herkunft steht am Zauber', () => {
  it('LAESST DIE REIHENFOLGE UNVERAENDERT — die Indizes stehen in jeder Aufzeichnung', () => {
    // **Die Zahlen sind abgeschrieben, und das ist hier richtig.** Sie stehen
    // so in den Vergleichsdateien; eine Liste, die sich aus der Umsetzung
    // selbst herleitet, koennte jede Umstellung mitmachen, ohne rot zu werden.
    expect(SPELLS.map((s) => s.id)).toEqual([
      'kindle',
      'smoulder',
      'pyre',
      'quench',
      'crust',
      'salve',
      'temper',
      'bind',
      'stall',
      'stay',
      'anchor',
      'vapour',
      'reflux',
      'canker',
      'fester',
      // Der Pruef-Zauber zur Wirkzeit (Issue #304), ganz hinten. Er gehoert
      // nicht zum Spiel, steht aber im Index-Raum — und genau der ist hier die
      // Aussage.
      'test-cast',
    ]);
  });

  it('gibt jedem Schulzauber seine Schule', () => {
    const erwartet: Record<string, string> = {
      kindle: 'CALCINATION',
      smoulder: 'CALCINATION',
      pyre: 'CALCINATION',
      crust: 'COAGULATION',
      salve: 'COAGULATION',
      temper: 'COAGULATION',
      bind: 'FIXATION',
      stall: 'FIXATION',
      stay: 'FIXATION',
      anchor: 'FIXATION',
      vapour: 'SUBLIMATION',
      reflux: 'SUBLIMATION',
      canker: 'PUTREFACTION',
      fester: 'PUTREFACTION',
    };
    for (const [id, schule] of Object.entries(erwartet)) {
      expect(SPELLS.find((s) => s.id === id)?.school, id).toBe(schule);
    }
  });

  it('laesst QUENCH ohne Schule — null heisst schulunabhaengig, nicht unbekannt', () => {
    // GAME-DESIGN 7.4: „Die drei schulunabhaengigen Sonderzauber". Ihn einer
    // Schule zuzuschlagen behauptete eine Zugehoerigkeit, die das Design
    // ausdruecklich verneint.
    expect(SPELLS.find((s) => s.id === 'quench')?.school).toBeNull();
  });

  it('deckt ALLE Zauber ab — keiner faellt durch', () => {
    // Ohne diesen Fall koennte ein neuer Zauber ohne Schule durchrutschen und
    // in der Ansicht lautlos in der falschen Gruppe landen.
    const ohne = SPELLS.filter((s) => s.school === null).map((s) => s.id);
    expect(ohne, 'nur die schulunabhaengigen duerfen null tragen').toEqual(['quench']);
  });

  it('nennt fuenf Schulen, nicht eine', () => {
    // Der eigentliche Befund, aus dem AC0 entstand: Vorher war alles
    // CALCINATION.
    const schulen = [...new Set(SPELLS.map((s) => s.school).filter((s) => s !== null))].sort();
    expect(schulen).toEqual([
      'CALCINATION',
      'COAGULATION',
      'FIXATION',
      'PUTREFACTION',
      'SUBLIMATION',
    ]);
  });
});
