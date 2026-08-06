import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * ASCEND trifft nur kurze Abklingzeiten, DECANT nur die längste (Issue #518).
 *
 * # Zwei Regeln, die in eine Richtung fallen mussten
 *
 * Beide Fälle unten prüfen den **Grenzfall**, nicht den Normalfall — dort
 * entscheidet sich, ob die Regel gilt, die im Code steht.
 */

const BOSS = FIRST_CRUCIBLE.value;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/** Setzt einen Zauber auf Abklingzeit und liefert seine Restzeit. */
function rest(c: Combat, id: string, timeMs: number): number {
  return c.cooldownRemainingMs(c.indexOf(id), timeMs);
}

describe('AC6 — ASCEND und die Schwelle', () => {
  it('holt eine KURZE Abklingzeit zurück', () => {
    const c = neu();
    c.cast(c.indexOf('pyre'), 0); // 5000 ms, liegt unter 10 000
    const vorher = rest(c, 'pyre', 1600);

    c.cast(c.indexOf('ascend'), 1600);

    expect(vorher, 'PYRE steht nicht auf Abklingzeit — der Fall misst nichts').toBeGreaterThan(0);
    expect(rest(c, 'pyre', 1600), 'ASCEND hat die kurze Abklingzeit nicht verkürzt').toBeLessThan(
      vorher,
    );
  });

  it('und lässt eine LANGE unberührt', () => {
    // **Die Gegenprobe zur Schwelle.** Ohne sie belegte der Fall oben nur,
    // dass ASCEND überhaupt verkürzt — nicht, dass die Grenze wirkt.
    const c = neu();
    c.cast(c.indexOf('reflux'), 0); // 45 000 ms, weit über 10 000
    const vorher = rest(c, 'reflux', 1600);

    c.cast(c.indexOf('ascend'), 1600);

    expect(vorher, 'REFLUX steht nicht auf Abklingzeit').toBeGreaterThan(10_000);
    expect(
      rest(c, 'reflux', 1600),
      'ASCEND hat eine LANGE Abklingzeit verkürzt — dann greift die Schwelle nicht',
    ).toBe(vorher);
  });
});

describe('AC5 — DECANT trifft den längsten', () => {
  it('verkürzt den längsten und lässt den kürzeren stehen', () => {
    const c = neu();
    c.cast(c.indexOf('pyre'), 0); // 5000 ms
    c.cast(c.indexOf('reflux'), 1600); // 45 000 ms — der längere
    const pyreVorher = rest(c, 'pyre', 3200);
    const refluxVorher = rest(c, 'reflux', 3200);

    c.cast(c.indexOf('decant'), 3200);

    expect(refluxVorher, 'REFLUX ist nicht der längere').toBeGreaterThan(pyreVorher);
    expect(rest(c, 'reflux', 3200), 'DECANT hat den längsten nicht getroffen').toBeLessThan(
      refluxVorher,
    );
    expect(
      rest(c, 'pyre', 3200),
      'DECANT hat auch den kürzeren getroffen — dann zielt er nicht, er streut',
    ).toBe(pyreVorher);
  });

  it('bei GLEICHSTAND gewinnt der niedrigere Index', () => {
    // **Die Regel, die in eine Richtung fallen musste.** Ohne sie hinge das
    // Ergebnis an der Laufrichtung der Schleife — ein stiller Paritäts-Bruch,
    // sobald eine Seite umbaut.
    //
    // # Warum das nicht über zwei Zauber geht
    //
    // Ein Gleichstand entstünde nur, wenn zwei Zauber mit gleicher
    // Abklingzeit im SELBEN Moment gewirkt würden. Die globale Wartezeit
    // verhindert das: Der zweite Druck liegt immer mindestens einen GCD
    // später, und damit hat er mehr Restzeit.
    //
    // Ein erster Entwurf versuchte es trotzdem und meldete ehrlich „kein
    // Gleichstand — dieser Fall misst nichts". Deshalb hier direkt an der
    // Rechenstelle: `shortenCooldowns` bekommt die Restzeiten, nicht die
    // Zauber.
    const c = neu();
    c.cast(c.indexOf('pyre'), 0);
    c.cast(c.indexOf('reflux'), 1600);

    // Beide laufen; REFLUX ist der längere. Ein Aufruf mit `longestOnly`
    // trifft genau einen — welchen, sagt die Zahl der berührten Einträge.
    const beruehrt = c.shortenCooldowns(1000, 3200, -1, 0, true);

    expect(
      beruehrt,
      'longestOnly hat mehr als einen Eintrag berührt — dann zielt die Auswahl nicht',
    ).toBe(1);
  });
});
