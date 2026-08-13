import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Der Stapel-Aufbau sammelt den Rest, statt ihn abzuschneiden (Issue #419).
 *
 * # Warum
 *
 * Bis zum 2026-08-13 stand in `addDecayStacks` ein `Math.floor` auf das
 * Produkt. Gemessen war der Aufbau-Weg damit **wirkungslos**: Alle drei
 * Quellen legen genau einen Stapel (canker 1, blight 1, rot 1), und bei
 * Basis 1 ändert ein Faktor erst ab 2,0 etwas.
 *
 * ```text
 *   floor(1 × 1,3) = 1     floor(1 × 1,5) = 1     floor(1 × 2,0) = 2
 * ```
 *
 * `putrefaction-build` trägt bei vollem Ausbau 1,5 — fünf Ränge, und kein
 * einziger Stapel mehr. Aufrunden verbietet der Datenkommentar (jeder erste
 * Rang schenkte sofort einen Stapel), also wartet der Rest jetzt auf den
 * nächsten Wurf.
 *
 * # Drei Eigenschaften, drei Fälle
 *
 * Sammeln allein genügt nicht — der Rest muss **je Quelle** laufen und mit dem
 * Kampf **enden**. Ohne das Erste entschiede die Reihenfolge im Kampf über den
 * Stapel, ohne das Zweite hinge der erste Wurf an der Vorgeschichte. Beides
 * wäre gegen die Planbarkeit aus Gesetz 6.
 */

/** Wirft CANKER sechsmal und gibt die Stapelzahl nach jedem Wurf. */
function verlauf(talents?: Record<string, number>): number[] {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  const canker = alsSpellIndex(combat.indexOf('canker'));
  const werte: number[] = [];
  for (let i = 0; i < 6; i++) {
    combat.cast(canker, i * 3000);
    werte.push(combat.decayStacks);
  }
  return werte;
}

describe('Die Zusicherung vor der Messung', () => {
  it('CANKER legt genau EINEN Stapel je Wurf', () => {
    // Der Träger des ganzen Vorhabens: Wäre die Basis größer, hätte schon der
    // alte `floor` gewirkt und es gäbe nichts zu beheben.
    expect(verlauf(), 'CANKER legt nicht mehr einen Stapel je Wurf').toEqual([1, 2, 3, 4, 5, 5]);
  });
});

describe('Der Rest wird gesammelt', () => {
  it('Faktor 1,5 legt bei jedem ZWEITEN Wurf einen Stapel mehr', () => {
    // 5 Ränge à 100 Promille. Rechnung je Wurf, in Promille:
    //
    //   Wurf 1   1500 + 0    -> 1 Stapel, Rest 500
    //   Wurf 2   1500 + 500  -> 2 Stapel, Rest 0
    //   Wurf 3   1500 + 0    -> 1 Stapel, Rest 500
    //
    // Also 1, 3, 4, dann die Obergrenze bei 5.
    expect(verlauf({ 'putrefaction-build': 5 })).toEqual([1, 3, 4, 5, 5, 5]);
  });

  it('Faktor 1,3 legt bei jedem VIERTEN — der Rest wächst langsamer', () => {
    // Der globale Weg, 5 × 60 Promille. Erst nach vier Würfen ist ein ganzer
    // Stapel zusammen (4 × 300 = 1200).
    expect(verlauf({ 'global-build': 5 })).toEqual([1, 2, 3, 5, 5, 5]);
  });
});

describe('Der Rest läuft JE QUELLE', () => {
  it('BLIGHT schiebt CANKERs Rest nicht über die Schwelle', () => {
    // **Ohne diese Trennung entschiede die Reihenfolge im Kampf.** Zwei
    // Quellen mit je Rest 500 ergäben zusammen einen Stapel, den keine von
    // beiden allein verdient hat — und welche ihn bekäme, hinge daran, wer
    // zufällig zuletzt geworfen hat.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: { 'putrefaction-build': 5 },
    });
    const canker = alsSpellIndex(combat.indexOf('canker'));
    const blight = alsSpellIndex(combat.indexOf('blight'));

    combat.cast(canker, 0); // 1 Stapel, CANKERs Rest 500
    combat.cast(blight, 3000); // 1 Stapel, BLIGHTs Rest 500
    expect(combat.decayStacks, 'ein Rest ist auf die andere Quelle übergelaufen').toBe(2);

    // Erst der ZWEITE Wurf derselben Quelle löst den Extra-Stapel aus.
    combat.cast(canker, 6000);
    expect(combat.decayStacks, 'CANKERs eigener Rest hat nicht ausgelöst').toBe(4);
  });
});

describe('Der Rest endet mit dem Kampf', () => {
  it('ein neuer Kampf beginnt bei null', () => {
    // **Sonst hinge der erste Wurf an der Vorgeschichte.** Zwei gleiche
    // Eröffnungen legten verschieden viele Stapel, je nachdem was vorher lief.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: { 'putrefaction-build': 5 },
    });
    const canker = alsSpellIndex(combat.indexOf('canker'));
    combat.cast(canker, 0);
    expect(combat.decayStacks, 'der erste Wurf legt nicht einen Stapel').toBe(1);

    combat.reset();
    combat.cast(canker, 0);
    expect(
      combat.decayStacks,
      'nach dem Zurücksetzen wirkt noch ein Rest aus dem alten Kampf',
    ).toBe(1);
  });
});
