import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';

/**
 * COAGULATIONs Schlüsseltalent (Issue #419, Bauart aus #418).
 *
 * # Was es ändert
 *
 * Ohne das Talent ist die Schutzschicht weg, sobald sie aufgebraucht ist. Mit
 * ihm entsteht im selben Moment eine **halbe** neue — einmal je gelegtem
 * Schild.
 *
 * # Die Begrenzung ist der Kern
 *
 * Ohne sie legte jeder Bruch nach, der nächste wieder, und die Schicht wäre
 * nicht mehr aufzubrauchen. Ein Schlüsseltalent baut die Rotation um, es
 * hebelt sie nicht aus — deshalb steht unten ein eigener Fall dafür.
 *
 * # Und der Schlag kommt trotzdem an
 *
 * Der letzte Fall ist der wichtigste: Ein Schlag, der die Schicht übersteigt,
 * muss den Rest beim Spieler abliefern. Beim Bauen stand hier ein früher
 * `return`, der genau das verschluckt hätte — aus der Nachlage wäre ein
 * Freischlag geworden.
 */

const MIT_TALENT = { 'coagulation-keystone': 1 };

/**
 * Legt eine Schicht und lässt den Boss zuschlagen — bis `bisMs`.
 *
 * **Über einen schlagenden Boss, nicht über einen eigenen Treffer-Aufruf.**
 * `hitPlayer` ist privat, und eine Methode, die es nur für den Test gäbe, wäre
 * ein Eingriff in den Kampf-Kern. `test-onslaught` wirkt Zauber mit 2000
 * Schaden; die Schicht trägt 900.
 *
 * **Die Rechnung für jeden Zeitpunkt steht daneben** — Start plus Wirkzeit
 * ergibt den Treffer, nicht der Start allein:
 *
 * ```text
 *   3000 ms Beginn + 2000 ms Wirkzeit = 5000 ms   erster Treffer
 *  10000 ms Beginn + 2000 ms Wirkzeit = 12000 ms  zweiter Treffer
 * ```
 */
function nachSchlaegen(bisMs: number, talents?: Record<string, number>) {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: bossById('test-onslaught') ?? FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  combat.cast(alsSpellIndex(combat.indexOf('crust')), 0);
  const gelegt = combat.shield;
  const hpVorher = combat.playerHp;
  for (let t = 0; t <= bisMs; t += 100) combat.stepPassive(t);
  return {
    gelegt,
    schildDanach: combat.shield,
    verloren: hpVorher - combat.playerHp,
    stapel: combat.shieldStacks,
  };
}

describe('Die Zusicherung vor der Messung', () => {
  it('CRUST legt überhaupt eine Schicht', () => {
    const lauf = nachSchlaegen(0);
    expect(lauf.gelegt, 'CRUST legt keine Schutzschicht — der Fall misst nichts').toBeGreaterThan(
      0,
    );
  });
});

describe('AC2 — die Nachlage', () => {
  it('ohne Talent ist die Schicht nach dem Bruch weg', () => {
    // 5500 ms: nach dem ersten Treffer (5000), vor dem zweiten (12000).
    const lauf = nachSchlaegen(5500);
    expect(lauf.gelegt).toBeGreaterThan(0);
    expect(lauf.schildDanach, 'ohne Talent darf nichts nachkommen').toBe(0);
    expect(lauf.stapel, 'die Stapel müssen mit weg sein').toBe(0);
  });

  it('mit Talent kommt die HÄLFTE nach', () => {
    const lauf = nachSchlaegen(5500, MIT_TALENT);
    expect(lauf.schildDanach, 'mit Talent muss eine halbe Schicht nachkommen').toBe(
      Math.floor(lauf.gelegt / 2),
    );
    expect(lauf.stapel, 'die Nachlage ist ein Stapel').toBe(1);
  });
});

describe('AC2 — und sie kommt nur EINMAL', () => {
  it('der zweite Bruch legt nichts mehr nach', () => {
    // **Ohne diesen Fall wäre die Schicht unaufbrauchbar.** Ein Talent, das
    // die Rotation aushebelt statt sie umzubauen, wäre kein Schlüsseltalent.
    //
    //   bis  5500 ms  ein Treffer  -> Nachlage steht
    //   bis 12500 ms  zwei Treffer -> Nachlage gebrochen, nichts mehr
    const nachEinem = nachSchlaegen(5500, MIT_TALENT);
    expect(
      nachEinem.schildDanach,
      'die erste Nachlage fehlt — der Fall misst nichts',
    ).toBeGreaterThan(0);

    const nachZweien = nachSchlaegen(12500, MIT_TALENT);
    expect(nachZweien.schildDanach, 'die zweite Nachlage darf es nicht geben').toBe(0);
  });
});

describe('AC3 — der Schlag kommt trotzdem an', () => {
  it('was die Schicht übersteigt, trifft den Spieler', () => {
    // **Der Fall zu meinem Beinahe-Fehler.** Ein früher Ausstieg nach der
    // Nachlage hätte den durchgekommenen Schaden verschluckt: Der Schild wäre
    // dann nicht eine halbe Schicht wert gewesen, sondern ein Freischlag.
    //
    // Gemessen bis 5500 ms — nach dem ersten Treffer (3000 + 2000 Wirkzeit).
    const ohne = nachSchlaegen(5500);
    const mit = nachSchlaegen(5500, MIT_TALENT);

    expect(
      ohne.verloren,
      'ohne Talent kommt gar nichts an — der Vergleich misst nichts',
    ).toBeGreaterThan(0);
    expect(mit.verloren, 'mit Talent kommt weniger an — die Nachlage verschluckt Schaden').toBe(
      ohne.verloren,
    );
  });
});

/**
 * Die Nachlage kommt aus dem Schild, der GELEGT wurde und JETZT bricht (#419).
 *
 * „Mit ihm entsteht im selben Moment eine halbe neue Schicht, einmal je
 * gelegtem Schild" — die Hälfte dessen, was zuletzt gelegt wurde und bricht.
 * Nicht die eines älteren, der längst abgelaufen ist.
 *
 * # Der Ablauf, an dem es schiefging
 *
 * Eine Schicht aus Umwandlung (TEMPER) wird nicht gelegt, sie wächst. Sie
 * schreibt weder die Dicke noch den Merker. Liegt sie allein und bricht, stand
 * dort noch die Dicke einer CRUST, die vorher UNGEBROCHEN abgelaufen war — und
 * deren Merker war frei, weil sie nie gebrochen ist:
 *
 * ```text
 *       0 ms  CRUST            990
 *    5000 ms  Schlag (320)     fängt sie auf, 670 bleiben
 *   12 000 ms                  CRUST läuft ab, ungebrochen
 *   13 000 ms  TEMPER, 14 500 ms KINDLE   -> 25 aus Umwandlung
 *   15 000 ms  Überladung (160) bricht die 25
 * ```
 *
 * Gegen `test-vessel`: der Schlag bei 5000 ms, die Überladung aus der
 * Energieleiste (7 je Sekunde, Schwelle 100) bei 15 000 ms.
 */
describe('die Nachlage gibt nur der gelegte Schild, der bricht', () => {
  const ABLAUF = new Map([
    [0, 'crust'],
    [13_000, 'temper'],
    [14_500, 'kindle'],
  ]);

  function ablauf(bisMs: number, talents?: Record<string, number>) {
    const combat = new Combat({
      school: CALCINATION.value,
      boss: bossById('test-vessel') ?? FIRST_CRUCIBLE.value,
      ...(talents === undefined ? {} : { talents }),
    });
    let crustAbgelaufen = -1;
    let vorher = 0;
    for (let t = 0; t <= bisMs; t += 10) {
      combat.stepPassive(t);
      const id = ABLAUF.get(t);
      if (id !== undefined) combat.cast(alsSpellIndex(combat.indexOf(id)), t);
      if (crustAbgelaufen < 0 && vorher > 0 && combat.shield === 0) crustAbgelaufen = t;
      vorher = combat.shield;
    }
    return { combat, crustAbgelaufen };
  }

  it('die CRUST läuft ungebrochen ab, und die Umwandlung legt eine eigene Schicht', () => {
    // **Die Zusicherung vor der Messung.** Bräche die CRUST unter dem Schlag,
    // wäre ihr Merker verbraucht, und der Fall unten wäre grün ohne die Regel.
    const bisAblauf = ablauf(12_500, MIT_TALENT);
    expect(bisAblauf.crustAbgelaufen, 'die CRUST verschwindet nicht durch Ablauf').toBe(12_010);
    expect(bisAblauf.combat.shieldAbsorbed, 'der Schlag bei 5000 ms trifft sie nicht').toBe(320);

    const vorDemBruch = ablauf(14_990, MIT_TALENT);
    expect(vorDemBruch.combat.converted, 'TEMPER wandelt nichts um').toBeGreaterThan(0);
    expect(vorDemBruch.combat.shield).toBe(vorDemBruch.combat.converted);
    expect(vorDemBruch.combat.shieldStacks, 'die Umwandlung ist keine gelegte Schicht').toBe(0);
  });

  it('bricht die Umwandlungs-Schicht, kommt NICHTS nach', () => {
    const mit = ablauf(15_000, MIT_TALENT);
    const ohne = ablauf(15_000);
    // Ohne Talent: Die Überladung bricht die Umwandlungs-Schicht ganz.
    expect(ohne.combat.shield, 'die Überladung bricht die Schicht nicht').toBe(0);
    expect(ohne.combat.shieldAbsorbed).toBe(320 + ohne.combat.converted);
    expect(mit.combat.shield, 'die abgelaufene CRUST legt ihre Hälfte nach').toBe(0);
    expect(mit.combat.shieldStacks).toBe(0);
  });

  it('liegt die Umwandlung AUF einer gelegten Schicht, gibt diese ihre Hälfte', () => {
    // **Die Gegenrichtung**, damit die Regel nicht zu viel sperrt: Bricht eine
    // gelegte CRUST, auf der Umwandlung liegt, ist es die CRUST, die bricht —
    // und ihre Hälfte kommt nach. Gegen `test-onslaught` (2000 bei 5000 ms).
    const combat = new Combat({
      school: CALCINATION.value,
      boss: bossById('test-onslaught') ?? FIRST_CRUCIBLE.value,
      talents: MIT_TALENT,
    });
    const plan = new Map([
      [0, 'crust'],
      [1500, 'temper'],
      [3000, 'kindle'],
    ]);
    let gelegt = 0;
    for (let t = 0; t <= 5000; t += 10) {
      combat.stepPassive(t);
      const id = plan.get(t);
      if (id !== undefined) combat.cast(alsSpellIndex(combat.indexOf(id)), t);
      if (t === 0) gelegt = combat.shield;
      if (t === 4990) {
        expect(combat.shield, 'auf der CRUST liegt keine Umwandlung').toBeGreaterThan(gelegt);
      }
    }
    expect(combat.shield, 'die gelegte CRUST gibt ihre Hälfte nicht').toBe(Math.floor(gelegt / 2));
  });
});
