import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

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

/** Legt eine Schicht und schlägt dann mit `schaden` zu. */
function nachSchlag(schaden: number, talents?: Record<string, number>) {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  const crust = alsSpellIndex(combat.indexOf('crust'));
  combat.cast(crust, 0);
  const gelegt = combat.shield;
  const vorher = combat.playerHp;
  combat.hitPlayer(schaden, 1500);
  return {
    gelegt,
    schildDanach: combat.shield,
    verloren: vorher - combat.playerHp,
    stapel: combat.shieldStacks,
  };
}

describe('Die Zusicherung vor der Messung', () => {
  it('CRUST legt überhaupt eine Schicht', () => {
    const lauf = nachSchlag(0);
    expect(lauf.gelegt, 'CRUST legt keine Schutzschicht — der Fall misst nichts').toBeGreaterThan(
      0,
    );
  });
});

describe('AC2 — die Nachlage', () => {
  it('ohne Talent ist die Schicht nach dem Bruch weg', () => {
    const lauf = nachSchlag(100000);
    expect(lauf.gelegt).toBeGreaterThan(0);
    expect(lauf.schildDanach, 'ohne Talent darf nichts nachkommen').toBe(0);
    expect(lauf.stapel, 'die Stapel müssen mit weg sein').toBe(0);
  });

  it('mit Talent kommt die HÄLFTE nach', () => {
    const lauf = nachSchlag(100000, MIT_TALENT);
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
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: MIT_TALENT,
    });
    combat.cast(alsSpellIndex(combat.indexOf('crust')), 0);

    // **Schläge, die die Schicht brechen, aber den Spieler NICHT töten.** Mit
    // 100000 war er nach dem ersten tot, der zweite tat gar nichts mehr — und
    // der Fall sah aus, als griffe die Begrenzung nicht.
    // **Doppelt so hart wie die Schicht dick ist.** Mit genau `gelegt` brach
    // sie nicht: WARD mindert den Schlag vorher, und 180 Punkte blieben
    // stehen — der Fall maß dann eine Nachlage, die noch gar nicht fällig war.
    const gelegt = combat.shield;
    combat.hitPlayer(gelegt * 2, 1500);
    const nachErstem = combat.shield;
    expect(nachErstem, 'die erste Nachlage fehlt — der Fall misst nichts').toBeGreaterThan(0);
    expect(
      combat.playerHp,
      'der Spieler ist tot — der zweite Schlag misst dann nichts',
    ).toBeGreaterThan(0);

    // **Deutlich mehr als die Nachlage**, weil WARD den Schlag vorher mindert:
    // Mit genau `nachErstem` blieben 36 Punkte stehen, die Schicht brach also
    // gar nicht — und der Fall meldete eine Nachlage, die keine war.
    combat.hitPlayer(gelegt * 2, 3000);
    expect(combat.shield, 'die zweite Nachlage darf es nicht geben').toBe(0);
    expect(
      combat.playerHp,
      'der Spieler ist tot — dann sagt der Fall nichts über die Grenze',
    ).toBeGreaterThan(0);
  });
});

describe('AC3 — der Schlag kommt trotzdem an', () => {
  it('was die Schicht übersteigt, trifft den Spieler', () => {
    // **Der Fall zu meinem Beinahe-Fehler.** Ein früher Ausstieg nach der
    // Nachlage hätte den durchgekommenen Schaden verschluckt: Der Schild wäre
    // dann nicht eine halbe Schicht wert gewesen, sondern ein Freischlag.
    // **Mehr als die Schicht trägt** (900) und weniger als tödlich (5000
    // Lebenspunkte). Bei 1000 fing sie noch alles ab — WARD mindert vorher,
    // und der Vergleich verglich zwei Nullen.
    const ohne = nachSchlag(2000);
    const mit = nachSchlag(2000, MIT_TALENT);

    expect(
      ohne.verloren,
      'ohne Talent kommt gar nichts an — der Vergleich misst nichts',
    ).toBeGreaterThan(0);
    expect(mit.verloren, 'mit Talent kommt kein Schaden an — die Nachlage verschluckt ihn').toBe(
      ohne.verloren,
    );
  });
});
