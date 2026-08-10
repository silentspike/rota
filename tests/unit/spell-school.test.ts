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
    //
    // # Warum hier zwei Zauber IN DER MITTE dazugekommen sind (Issue #268)
    //
    // Bis #268 galt eine ungeschriebene Regel: Neue Zauber wurden **hinten
    // angehaengt**, auch wenn sie logisch dazwischengehoerten. Der Grund steht
    // dreimal als Kommentar in `data.ts` und nirgends sonst:
    //
    // > Die Reihenfolge bestimmt die Indizes, und die stehen in jeder
    // > aufgezeichneten Runde — vorne einzufuegen deutete jede alte
    // > Aufzeichnung STILL um.
    //
    // Das Wort ist **still**. Genau das ist es seit `tools/ac6.sh` nicht mehr:
    // Der Befehl sagt Feld fuer Feld, was sich bewegt hat, und trennt
    // Index-Verschiebungen von echten Verhaltensaenderungen.
    //
    // Deshalb stehen `bellows` und `searing` bei ihrer Schule statt hinten.
    // Die Regel bestand aus einem Grund, der Grund ist abgedeckt — und eine
    // Regel, die dafuer Kalzinations-Zauber hinter PUTREFACTION zwingt, waere
    // teurer als das Problem.
    //
    // **Dieser Fall bleibt trotzdem, und er soll weiter rot werden.** Er ist
    // die einzige Stelle, an der eine Umstellung ueberhaupt auffaellt; die
    // Entscheidung war, sie zuzulassen, nicht sie unsichtbar zu machen.
    expect(SPELLS.map((s) => s.id)).toEqual([
      'kindle',
      'smoulder',
      'pyre',
      'bellows',
      'searing',
      'cinder',
      'slag',
      'ember',
      'pyroclasm',
      // Issue #540 — FLASHPOINT, bei seiner Schule.
      'flashpoint',
      'quench',
      // Issue #561 — PURGE senkt das Verhaengnis, neben quench weil beide
      // schulunabhaengig sind.
      'purge',
      'crust',
      'salve',
      'temper',
      'scab',
      'plate',
      'harden',
      // Issue #540 — QUICKSILVER, bei seiner Schule.
      'quicksilver',
      // Issue #481 — POULTICE, Heilung ueber Zeit (Weg 2 der Schule).
      'poultice',
      // Issue #481 — MEND, ebenfalls Weg 2 und in der Reihenfolge des Rasters.
      // Er schiebt CRUCIBLE SKIN um eins nach hinten; das geschieht hier und
      // in `spell-order.json` sichtbar, nicht still (wie #268, #479).
      'mend',
      // Issue #481 — SUTURE verbindet Weg 1 und Weg 2; er steht hinter MEND,
      // weil er in `coagulation.json` dort steht.
      'suture',
      // Issue #481 — RIPOSTE gibt eingehenden Schaden zurueck; er steht hinter
      // SUTURE, weil er in `coagulation.json` dort steht.
      'riposte',
      // Issue #481 — CRUCIBLE SKIN, hinter QUICKSILVER wie in
      // `coagulation.json`. Diese Liste und `spell-order.json` führen
      // dieselbe Reihenfolge; sie steht an zwei Stellen, damit eine
      // Verschiebung nicht nur dort auffällt, wo sie gemacht wurde.
      'crucible-skin',
      'bind',
      'stall',
      'stay',
      'anchor',
      'wedge',
      'tether',
      'capstone',
      'clasp',
      'latch',
      // Issue #540 — GRIPLOCK, bei seiner Schule.
      'griplock',
      // Issue #478 — PIN, am ENDE von `fixation.json`. Er schiebt damit alle
      // folgenden Schulen um einen Index; jede Aufzeichnung wird neu erzeugt.
      'pin',
      'vapour',
      'reflux',
      'wisp',
      'alembic',
      'ascend',
      'decant',
      'efflux',
      // Issue #540 — DISTIL, bei seiner Schule.
      'distil',
      // Issue #480 — AETHER, hinten in `sublimation.json`.
      'aether',
      'canker',
      'fester',
      'blight',
      'rot',
      'sepsis',
      // Issue #540 — VIRULENCE, bei seiner Schule.
      'virulence',
      // Issue #479 — MIASMA, am ENDE von `putrefaction.json`. Mein erster
      // Einbau setzte ihn zwischen die Füller, wo er fachlich hingehört —
      // und verschob damit den Index-Raum. Dieser Fall und
      // `spell-order.test.ts` haben es sofort gemeldet.
      'miasma',
      // Issue #479 — NECROSIS, der Entlader, ebenfalls am Ende.
      'necrosis',
      // Issue #479 — DECAY, der neunte der Schule, ebenfalls am Ende.
      'decay',
      // Issue #479 — PUTRID BLOOM, ebenfalls am Ende.
      'putrid-bloom',
      'test-cast',
      'test-buff',
      'test-control',
      'test-heat',
      'test-tick',
      'test-stack',
      'test-cdreduce',
      'test-convert',
      'test-cap',
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
    expect(ohne, 'nur die schulunabhaengigen duerfen null tragen').toEqual(['quench', 'purge']);
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
