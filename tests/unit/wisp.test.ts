import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * WISP — der Füller, den SUBLIMATION nicht hatte (Issue #480).
 *
 * # Warum die Schule ihn braucht
 *
 * Beide vorhandenen Zauber machen **keinen Schaden**. Der Balance-Läufer hat
 * sie an zwei Bössen geprüft: Sie kommen in *keiner* optimalen Rotation vor.
 * Man konnte die Schule nicht spielen, nur einstreuen.
 *
 * WISP verkürzt wie VAPOUR und teilt dabei Schaden aus. Damit gibt es
 * erstmals einen Grund, sie zu drücken.
 */

const BOSS = FIRST_CRUCIBLE.value;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

describe('AC2 — WISP ist im Kampf wirksam', () => {
  it('teilt Schaden aus', () => {
    const c = neu();
    const vorher = c.damageDealt;
    c.cast(c.indexOf('wisp'), 0);
    expect(c.damageDealt - vorher, 'WISP macht keinen Schaden').toBeGreaterThan(0);
  });

  it('und verkürzt eine laufende Abklingzeit', () => {
    // Erst PYRE auf Abklingzeit setzen, dann WISP drücken.
    const c = neu();
    const pyre = c.indexOf('pyre');
    c.cast(pyre, 0);
    const restVorher = c.cooldownRemainingMs(pyre, 1600);

    c.cast(c.indexOf('wisp'), 1600);
    const restNachher = c.cooldownRemainingMs(pyre, 1600);

    expect(restVorher, 'PYRE steht nicht auf Abklingzeit — der Fall misst nichts').toBeGreaterThan(
      0,
    );
    expect(restNachher, 'WISP hat nichts verkürzt').toBeLessThan(restVorher);
  });
});

describe('AC3 — Gegenprobe: ohne WISP ändert sich nichts', () => {
  it('ein anderer Zauber verkürzt die Abklingzeit nicht', () => {
    // **Ohne diese Zeile belegte AC2 nur, dass irgendein Druck etwas tut.**
    const c = neu();
    const pyre = c.indexOf('pyre');
    c.cast(pyre, 0);
    const vorher = c.cooldownRemainingMs(pyre, 1600);

    c.cast(c.indexOf('kindle'), 1600);

    expect(
      c.cooldownRemainingMs(pyre, 1600),
      'KINDLE verkürzt die Abklingzeit — dann kommt die Verkürzung nicht von WISP',
    ).toBe(vorher);
  });
});
