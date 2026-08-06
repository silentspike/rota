import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

/**
 * Die Sperre gegen Ändern im Kampf greift auch bei Kampfbeginn zur Zeit 0
 * (Issue #433).
 *
 * # Der Befund
 *
 * Drei Setzer trugen wörtlich dieselbe Zeile:
 *
 * ```ts
 * if (this.stateValue === FIGHT_RUNNING && this.fightStartedAtMs > 0) return;
 * ```
 *
 * `fightStartedAtMs` ist vor dem Kampf `0` und wird bei `beginFight` auf die
 * Startzeit gesetzt. Beginnt ein Kampf **zur Zeit 0**, bleibt der Wert `0` —
 * und der Ausdruck `> 0` liest das als „noch nicht begonnen".
 *
 * Im Spiel läuft vorher ein Countdown, also ist die Startzeit dort nie 0. **Im
 * Prüfstand ist 0 der naheliegende Wert.** Die Sperre war also genau dort
 * offen, wo sie am ehesten gebraucht wird.
 *
 * # Warum drei Prüffälle und nicht einer
 *
 * Die Lücke ist dreimal dieselbe, die **Wirkung** nicht:
 *
 * | Stelle | Bauart | Was bei offener Sperre passiert |
 * | --- | --- | --- |
 * | `setWorn` | Vollsperre | die Schadensrechnung ändert sich mitten im Lauf |
 * | `setTalents` | Vollsperre | dasselbe |
 * | `setPlayerLevel` | **Teilsperre** | nur die Lebenspunkte werden aufgefüllt |
 *
 * Ein Prüffall, der nur `setWorn` abdeckt, ließe offen, ob die Teilsperre noch
 * eine Teilsperre ist — und genau die ist die, die man beim Reparieren
 * versehentlich zur Vollsperre macht.
 */

/**
 * Der Schaden eines Wirkens — **die Wirkung, nicht das Feld**.
 *
 * `factors` ist privat, und ein Lesezugriff nur für einen Prüffall wäre eine
 * Schnittstelle, die dem Spiel nichts bringt. Wichtiger: Der Schaden belegt,
 * dass der Wert **ankommt**, das Feld nur, dass er berechnet wurde. Dieselbe
 * Wahl wie bei #417.
 */
function schadenEinesWirkens(combat: Combat, timeMs: number): number {
  const vorher = combat.damageDealt;
  combat.cast(combat.indexOf('kindle'), timeMs);
  return combat.damageDealt - vorher;
}

function imKampfAbZeitNull(): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  // **Genau der Fall aus dem Befund.** Nicht 1, nicht 10 — null.
  combat.beginFight(0);
  return combat;
}

const AUSRUESTUNG = { ...NOTHING_WORN, essence: 300, vessel: 500 };

describe('AC1/AC2 — setWorn ist im Kampf ab Zeit 0 gesperrt', () => {
  it('die Faktoren bleiben, wo sie waren', () => {
    const combat = imKampfAbZeitNull();
    const vorher = schadenEinesWirkens(combat, 0);

    combat.setWorn(AUSRUESTUNG);

    expect(
      schadenEinesWirkens(combat, 1500),
      'ESSENCE hat sich mitten im Kampf geändert — bereits gefallene Treffer ' +
        'wären anders gerechnet als die folgenden',
    ).toBe(vorher);
  });

  it('und außerhalb des Kampfes geht es sehr wohl', () => {
    // **Der Gegenfall.** Ohne ihn zeigte der Fall oben nur, dass `setWorn`
    // nichts tut — nicht, dass die Sperre wirkt.
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    ohne.beginFight(0);
    const vorher = schadenEinesWirkens(ohne, 0);

    const mit = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    mit.setWorn(AUSRUESTUNG); // VOR beginFight — hier darf es wirken
    mit.beginFight(0);

    expect(schadenEinesWirkens(mit, 0)).not.toBe(vorher);
  });
});

describe('AC1/AC2 — setTalents ist im Kampf ab Zeit 0 gesperrt', () => {
  it('der Talent-Faktor bleibt, wo er war', () => {
    const combat = imKampfAbZeitNull();
    const vorher = schadenEinesWirkens(combat, 0);

    combat.setTalents({ 'calcination-build': 3 });

    expect(
      schadenEinesWirkens(combat, 1500),
      'der Talent-Faktor hat sich mitten im Kampf geändert',
    ).toBe(vorher);
  });

  it('und außerhalb des Kampfes geht es sehr wohl', () => {
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    ohne.beginFight(0);
    const vorher = schadenEinesWirkens(ohne, 0);

    const mit = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    mit.setTalents({ 'calcination-build': 3 }); // VOR beginFight
    mit.beginFight(0);

    expect(schadenEinesWirkens(mit, 0)).not.toBe(vorher);
  });
});

describe('AC3 — setPlayerLevel bleibt eine TEILsperre', () => {
  it('die Lebenspunkte werden im Kampf nicht aufgefüllt', () => {
    const combat = imKampfAbZeitNull();
    const vorher = combat.playerHp;

    combat.setPlayerLevel(30);

    expect(
      combat.playerHp,
      'die Lebenspunkte wurden mitten im Kampf aufgefüllt — geschenktes Leben',
    ).toBe(vorher);
  });

  it('aber die Obergrenze wird sehr wohl gesetzt — das ist die andere Hälfte', () => {
    // **Der Fall, den eine zu gründliche Reparatur kaputtmacht.** Wer die
    // Sperrzeile nach oben zieht, macht aus der Teilsperre eine Vollsperre:
    // Stufe und `vesselMax` beschreiben den Spieler und dürfen sich auch im
    // Kampf ändern. Nur das Auffüllen darf es nicht.
    // `vesselMax` ist privat. Sichtbar wird die Obergrenze über `reset()`:
    // Es füllt die Lebenspunkte auf sie auf. Steht sie nach einem
    // `setPlayerLevel` im Kampf höher als vorher, ist die zweite Hälfte der
    // Teilsperre durchgelaufen.
    const vergleich = imKampfAbZeitNull();
    vergleich.reset();
    const vorher = vergleich.playerHp;

    const combat = imKampfAbZeitNull();
    combat.setPlayerLevel(30);
    combat.reset();

    expect(
      combat.playerHp,
      'vesselMax wurde im Kampf NICHT gesetzt — aus der Teilsperre ist eine ' +
        'Vollsperre geworden',
    ).toBeGreaterThan(vorher);
  });
});

describe('Die Ursache selbst — Zeit 0 ist der einzige Unterschied', () => {
  it('ein Kampf ab Zeit 10 war schon vorher gesperrt', () => {
    // **Der Beleg, dass die Lücke wirklich an der Null hing** und nicht an
    // etwas anderem: Bei jeder anderen Startzeit griff die alte Bedingung.
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.beginFight(10);
    const vorher = schadenEinesWirkens(combat, 10);

    combat.setWorn(AUSRUESTUNG);

    expect(schadenEinesWirkens(combat, 1510)).toBe(vorher);
  });
});
