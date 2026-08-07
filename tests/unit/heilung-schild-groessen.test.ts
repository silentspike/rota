import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, bossById, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Die GRÖSSEN von Heilung und Schild — Issue #592.
 *
 * # Warum diese beiden lange unsichtbar waren
 *
 * Bis #576 maß die Hebel-Messung ausschließlich `expected.damage`. `salve`
 * heilt und `plate` schirmt ab — **eine Null im Schadenshebel war dort die
 * Definition des Zaubers, kein Befund.** Erst mit `healed` und `damageTaken`
 * als eigenen Messgrößen kamen die wahren Werte heraus:
 *
 * ```text
 *   heal.amount   salve   50,00 %  über healed
 *   shield.amount scab    33,33 %  über damageTaken
 * ```
 *
 * Beide standen davor auf „NUR FRISCHE": Man konnte den Wert halbieren, und
 * nur die Frischeprüfung meldete sich — die nach einem `parity:export`
 * verstummt.
 *
 * # Die Lage muss hergestellt werden, sonst misst der Fall nichts
 *
 * Eine Heilung bei vollem Leben heilt **null**. Gemessen: Gegen den
 * Standard-Boss nimmt der Spieler in zehn Sekunden gar keinen Schaden, gegen
 * `test-caster` genau 96 — und `salve` heilte dort 96 statt 1500, weil mehr
 * gar nicht fehlte. Ein Prüffall mit dieser Lage wäre gegen `heal.amount
 * 1500 → 750` **grün geblieben**.
 *
 * `test-onslaught` teilt genug aus: nach 10 000 ms sind 1600 Schaden
 * gefallen, der Spieler steht bei 3400 von 5000. Dort deckt die Heilung ihren
 * vollen Betrag ab, ohne an der Obergrenze zu klemmen.
 */

// **Nicht `bossById(...)!`.** Fehlt der Boss, soll die Datei mit dieser
// Meldung brechen statt mit „cannot read properties of undefined" tief im
// Kampf — dort wäre die Ursache nicht mehr abzulesen.
const NEHMEND =
  bossById('test-onslaught') ??
  (() => {
    throw new Error('Boss test-onslaught fehlt — ohne ihn nimmt der Spieler keinen Schaden');
  })();

/** Der Zeitpunkt, an dem genug fehlt und noch Leben da ist. */
const NACH_MS = 10000;

function angeschlagen(): Combat {
  const c = new Combat({ school: CALCINATION.value, boss: NEHMEND });
  for (let t = 0; t <= NACH_MS; t += 10) c.stepPassive(t);
  return c;
}

describe('SALVE heilt 1500 — nicht „irgendetwas"', () => {
  /**
   * **Die Zusicherung vor der Messung.** Steht der Spieler bei vollem Leben,
   * heilt SALVE null, und der Fall darunter wäre grün mit der Aussage „die
   * Heilung wirkt nicht" — ein erfundener Fehler.
   */
  it('der Spieler ist überhaupt angeschlagen', () => {
    const c = angeschlagen();
    expect(c.playerHp, 'kein Schaden gefallen — dann misst der Fall unten nichts').toBeLessThan(
      5000,
    );
    expect(
      5000 - c.playerHp,
      'es fehlen weniger als 1500 — die Heilung klemmt an der Obergrenze und ihr Betrag ist unsichtbar',
    ).toBeGreaterThanOrEqual(1500);
  });

  it('und zwar um 1500, nicht um irgendeinen Betrag', () => {
    const c = angeschlagen();
    c.cast(c.indexOf('salve'), NACH_MS);
    expect(c.healed, 'SALVE heilt nicht die 1500 aus dem Design').toBe(1500);
  });

  /** Die Gegenprobe: Ohne SALVE bewegt sich nichts. */
  it('ohne SALVE bleibt geheilt bei null', () => {
    expect(angeschlagen().healed, 'es wird geheilt, ohne dass jemand SALVE wirkt').toBe(0);
  });
});

describe('SCAB legt 200 Schutz auf — je Ladung', () => {
  it('eine Ladung sind 200', () => {
    const c = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    c.cast(c.indexOf('scab'), 0);
    expect(c.shield, 'SCAB legt nicht die 200 aus dem Design auf').toBe(200);
  });

  /**
   * `shield.maxStacks` ist bereits über `coagulation-scab.test.ts` geprüft.
   * Dieser Fall nagelt den BETRAG fest — die Mutation `200 → 100` bewegt
   * 33,33 % des erlittenen Schadens einer Aufzeichnung und blieb bis heute
   * unbemerkt.
   */
  it('zwei Ladungen sind 400 — der Betrag summiert sich', () => {
    const c = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    c.cast(c.indexOf('scab'), 0);
    c.cast(c.indexOf('scab'), 1500);
    expect(c.shield, 'die zweite Ladung legt nicht denselben Betrag drauf').toBe(400);
  });
});
