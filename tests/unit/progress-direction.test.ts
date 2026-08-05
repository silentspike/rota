/**
 * Der Fortschritt kennt die Richtung des Bosses (Issue #256, AC1 und AC5).
 *
 * DIE EINE ENTSCHEIDUNG, DREIMAL ANGEWANDT
 * ----------------------------------------
 * Worker-2 hat vor dem Umbau gemessen, dass an DREI Stellen dieselbe Annahme
 * steckt — „größer ist besser" —, an einer davon als ausdrücklicher Kommentar.
 * Solange `progressOf` sie einhält, stimmen `progress-mark.ts` und
 * `run-compare.ts` mit, ohne dass jemand dort etwas ändert.
 *
 * | | Fortschritt | 100 % heißt |
 * | --- | --- | --- |
 * | gewöhnlich | Schaden am Ring | der Boss ist gefallen |
 * | mit `runaway` | überstandene Frist | die Frist ist überstanden |
 *
 * AC5 IST DER GEFÄHRLICHSTE FALL
 * ------------------------------
 * „Bei einem gewöhnlichen Boss ändert sich nichts" lässt sich erfüllen, ohne
 * dass etwas geprüft wird — ein Fall mit 0 Schaden und 0 Zeit gibt in beiden
 * Rechnungen 0. Die Fälle unten stehen deshalb an Zuständen, in denen die
 * beiden Rechnungen **verschiedene** Zahlen liefern würden: Dann und nur dann
 * zeigt der Vergleich etwas.
 *
 * Das ist die Frage aus der vierten Gestalt: Könnte dieser Fall überhaupt
 * einen Unterschied zeigen?
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { progressOf } from '../../src/core/summary.js';

const RUNAWAY = bossById('test-runaway');
if (RUNAWAY === undefined) throw new Error('data/bosses/test-runaway.json fehlt.');

/** Einen Lauf bis `bisMs` führen und den Kampf zurückgeben. */
function laufen(boss: typeof FIRST_CRUCIBLE.value, bisMs: number, spellId?: string): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const index = spellId === undefined ? -1 : combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.checkDefeat(t)) break;
    if (index >= 0 && combat.gcd.isReady(t) && combat.isReady(index, t)) combat.cast(index, t);
  }
  return combat;
}

describe('AC1 — bei einem runaway-Boss zählt die überstandene Frist', () => {
  it('trifft die gerechneten Zahlen aus den beiden gemessenen Läufen', () => {
    // **Die Zahlen standen vor dem Umbau fest** (Vorschlag an den
    // Orchestrator, an echten Browser-Läufen gerechnet):
    //   Niederlage bei 17 s von 20 s  ->  85 %
    //   Sieg bei 20 s                 -> 100 %
    const frist = RUNAWAY.overpressureMs;
    expect(frist).toBe(20_000);

    const combat = laufen(RUNAWAY, 25_000);
    expect(combat.endedAt, 'ohne Eingriff kippt der Ring bei 17 s').toBe(17_000);
    expect(progressOf(combat, combat.endedAt)).toBeCloseTo(85, 5);
  });

  it('steht bei 100 %, wenn die Frist überstanden ist', () => {
    const combat = laufen(RUNAWAY, 25_000, 'crust');
    expect(combat.endedAt).toBe(RUNAWAY.overpressureMs);
    expect(progressOf(combat, combat.endedAt)).toBe(100);
  });

  it('SAGT NICHT MEHR DAS GEGENTEIL — die alte Rechnung im Vergleich', () => {
    // **Der Befund, aus dem #256 entstand.** Die alte Formel zählte den
    // Schaden am Ring; bei diesem Boss ist der der Weg zur Niederlage.
    const verloren = laufen(RUNAWAY, 25_000);
    const gewonnen = laufen(RUNAWAY, 25_000, 'crust');

    const altVerloren = (100 * verloren.damageDealt) / RUNAWAY.hp;
    const altGewonnen = (100 * gewonnen.damageDealt) / RUNAWAY.hp;

    // Alt: die Niederlage stand HÖHER als der Sieg.
    expect(altVerloren).toBeGreaterThan(altGewonnen);
    // Neu: der Sieg steht höher — wie bei jedem anderen Boss.
    expect(progressOf(gewonnen, gewonnen.endedAt)).toBeGreaterThan(
      progressOf(verloren, verloren.endedAt),
    );
  });

  it('wächst über den Lauf, statt zu fallen', () => {
    // Ein Fortschritt, der sinkt, während der Spieler durchhält, wäre
    // dieselbe verkehrte Aussage in klein.
    const combat = new Combat({ school: CALCINATION.value, boss: RUNAWAY });
    combat.beginFight(0);
    let vorher = -1;
    for (let t = 0; t <= 15_000; t += 1000) {
      combat.stepPassive(t);
      const jetzt = progressOf(combat, t);
      expect(jetzt, `bei ${t} ms`).toBeGreaterThan(vorher);
      vorher = jetzt;
    }
  });
});

describe('AC5 — bei einem gewöhnlichen Boss ändert sich nichts', () => {
  it('rechnet weiter den Schaden am Ring', () => {
    // **An einem Zustand, der einen Unterschied zeigen KANN — und das war nicht
    // der erste Versuch.** Mit KINDLE gedrückt liegen die beiden Rechnungen
    // nach 15 s bei 26,2 % und 25,0 %: Der Schmelztiegel ist so ausgelegt, dass
    // eine gute Rotation ihn etwa zur Frist besiegt, also laufen Schaden und
    // Zeit fast proportional. Die Zusicherung unten hat das gefangen.
    //
    // Ohne Aktion trennen sie sich deutlich: 0 % Schaden gegen 25 % Zeit.
    const combat = laufen(FIRST_CRUCIBLE.value, 15_000);
    const nachSchaden = (100 * combat.damageDealt) / FIRST_CRUCIBLE.value.hp;
    const nachZeit = (100 * 15_000) / FIRST_CRUCIBLE.value.overpressureMs;

    expect(
      Math.abs(nachSchaden - nachZeit),
      'die beiden Rechnungen müssen sich hier unterscheiden, sonst prüft der Fall nichts',
    ).toBeGreaterThan(20);

    expect(progressOf(combat, 15_000)).toBeCloseTo(nachSchaden, 5);
    expect(progressOf(combat, 15_000)).not.toBeCloseTo(nachZeit, 1);
  });

  it('bleibt bei 100 %, wenn der Boss fällt', () => {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.beginFight(0);
    combat.setBossHpRemaining(0);
    expect(progressOf(combat, 5_000)).toBe(100);
  });

  it('trägt kein runaway — sonst prüfte der Gegenfall den falschen Boss', () => {
    expect(FIRST_CRUCIBLE.value.runaway).toBeNull();
    expect(RUNAWAY.runaway).not.toBeNull();
  });
});

describe('Die Ränder', () => {
  it('gibt 0 statt einer Division durch null', () => {
    // Ein Boss ohne Frist hätte kein Ziel; ein Boss ohne Lebenspunkte auch.
    const ohneFrist = { ...RUNAWAY, overpressureMs: 0 };
    const combat = new Combat({ school: CALCINATION.value, boss: ohneFrist as never });
    combat.beginFight(0);
    expect(progressOf(combat, 5_000)).toBe(0);
  });

  it('deckelt bei 100, statt darüber zu gehen', () => {
    const combat = laufen(RUNAWAY, 25_000, 'crust');
    expect(progressOf(combat, 999_999)).toBe(100);
  });
});
