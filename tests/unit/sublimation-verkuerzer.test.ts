/**
 * Die vier Verkürzer von SUBLIMATION (Issue #518, E3-3).
 *
 * # Vier Lücken auf der obersten Ebene
 *
 * Gemessen über die **oberste** Ebene der Zauber-Daten, die
 * `tools/groessen-mutation.py` nicht anfasst:
 *
 * ```text
 *   cooldownReductionMs   vapour    1000 -> 500     NUR FRISCHE
 *   cooldownReductionMs   decant    2000 -> 1000    NUR FRISCHE
 *   cooldownReductionMs   ascend   10000 -> 5000    SCHWEIGT
 *   cooldownResetCharges  alembic      2 -> 1       NUR FRISCHE
 * ```
 *
 * Die **Auswahl**, die jeder trifft, ist längst geprüft:
 * `ascend-decant.test.ts` deckt `cooldownReductionBelowMs` und
 * `cooldownReductionLongestOnly` ab. Nur **wie viel** sie abziehen, prüfte
 * niemand — dasselbe Muster wie bei den Schild-Zaubern, nur eine Ebene höher.
 *
 * # Warum ASCEND schwieg — der Abzug deckelt sich selbst
 *
 * Er zieht 10000 ms ab, aber nur bei Zaubern mit **weniger als** 10000 ms
 * Restzeit (`cooldownReductionBelowMs`). Jeder getroffene Zauber landet damit
 * auf **0**, egal wie groß der Abzug ist:
 *
 * ```text
 *                   plate    harden   purge
 *   vorher          15500     7000    6000
 *   ASCEND          15500        0       0
 *   Abzug halbiert  15500     2000    1000   <- erst hier wird es sichtbar
 * ```
 *
 * Eine 0 sieht aus wie „vollständig zurückgesetzt". Dass ein **Abzug**
 * dahintersteckt und keine Rücksetzung, zeigt erst der Lauf mit halbierter
 * Zahl. Genau deshalb schwieg die Mutation über die ganze Suite.
 *
 * # Alle vier an ihrer Wirkung, mit gemessener Gegenprobe
 *
 * ```text
 *                    Bestand              halbiert
 *   VAPOUR     14500/6000/5000      15000/6500/5500
 *   DECANT     13500/7000/6000      14500/7000/6000
 *   ASCEND     15500/   0/   0      15500/2000/1000
 *   ALEMBIC    2 Ladungen, armed    1 Ladung, nicht mehr armed
 * ```
 */
import { describe, expect, it } from 'vitest';

import { CAST_OK, Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Drei Zauber mit unterschiedlich langer Abklingzeit als Messfeld.
 *
 * **Unterschiedlich lang ist Absicht**: PLATE liegt über ASCENDs Schwelle,
 * die anderen darunter, und DECANT trifft nur den längsten. Mit drei gleich
 * langen Zaubern wären alle drei Auswahlregeln ununterscheidbar.
 */
const VORLAUF = ['plate', 'harden', 'purge'] as const;

/** Restzeiten der drei Vorlauf-Zauber vor und nach dem Wurf von `id`. */
function abzuege(id: string | null): { vor: string; nach: string } {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  let zeit = 0;
  for (const v of VORLAUF) {
    combat.stepPassive(zeit);
    expect(combat.cast(alsSpellIndex(combat.indexOf(v)), zeit).result, `${v} kam nicht durch`).toBe(
      CAST_OK,
    );
    zeit += 1500;
  }
  combat.stepPassive(zeit);

  const rest = (): string =>
    VORLAUF.map((v) => combat.cooldownRemainingMs(alsSpellIndex(combat.indexOf(v)), zeit)).join(
      '/',
    );

  const vor = rest();
  if (id !== null) {
    expect(
      combat.cast(alsSpellIndex(combat.indexOf(id)), zeit).result,
      `${id} kam nicht durch`,
    ).toBe(CAST_OK);
  }
  return { vor, nach: rest() };
}

describe('das Messfeld steht — sonst misst nichts darunter etwas', () => {
  it('die drei Vorlauf-Zauber laufen mit 15500/7000/6000', () => {
    // Wird einer davon umbalanciert, gehören die Zahlen unten nachgezogen —
    // dieser Fall sagt dann, WARUM sie sich bewegt haben.
    expect(abzuege(null).vor, 'die Abklingzeiten des Vorlaufs haben sich geändert').toBe(
      '15500/7000/6000',
    );
  });

  it('und ohne Verkürzer bewegt sich nichts', () => {
    const { vor, nach } = abzuege(null);
    expect(nach).toBe(vor);
  });
});

describe('VAPOUR zieht überall 1000 ab (cooldownReductionMs)', () => {
  it('alle drei verlieren je eine Sekunde', () => {
    expect(abzuege('vapour').nach, 'VAPOUR zieht nicht 1000 ab').toBe('14500/6000/5000');
  });
});

describe('DECANT zieht 2000 ab — aber nur beim längsten (cooldownReductionMs)', () => {
  it('PLATE verliert 2000, die anderen nichts', () => {
    // Die Auswahl (`longestOnly`) ist anderswo geprüft; hier zählt die **Zahl**.
    // Halbiert stünde `14500/7000/6000`.
    expect(abzuege('decant').nach, 'DECANT zieht nicht 2000 vom längsten ab').toBe(
      '13500/7000/6000',
    );
  });
});

describe('ASCEND zieht 10000 ab, unterhalb seiner Schwelle (cooldownReductionMs)', () => {
  it('was darunter liegt, fällt auf null — was darüber liegt, bleibt', () => {
    expect(abzuege('ascend').nach, 'ASCEND trifft nicht die beiden unter 10000').toBe('15500/0/0');
  });

  it('und die 0 ist ein ABZUG, keine Rücksetzung', () => {
    // **Der Grund für das Schweigen der Mutation.** Beide Zauber landen auf 0,
    // weil 10000 größer ist als ihre Restzeit — jede kleinere Zahl über 7000
    // täte dasselbe. Sichtbar wird der Unterschied erst bei einer Halbierung
    // (`15500/2000/1000`), gemessen mit mutierter Datei.
    //
    // Der Fall hier hält deshalb fest, dass PLATE **unberührt** bleibt: Ein
    // echter Rücksetzer kennte keine Schwelle und nähme ihn mit.
    expect(abzuege('ascend').nach.startsWith('15500/'), 'ASCEND greift über seine Schwelle').toBe(
      true,
    );
  });
});

describe('ALEMBIC gibt zwei Ladungen zum Zurücksetzen (cooldownResetCharges)', () => {
  /**
   * Stellt scharf und wählt **einmal** — steht der Auswahlmodus danach noch?
   *
   * `armResetIfNeeded` liegt in `Session.apply`, nicht in `Combat.cast`
   * (dokumentiert an `resetArmedForValue`). Der Fall ruft es deshalb selbst;
   * gemessen wird die Leitung von der Datei in `resetChargesLeft`.
   */
  function nachEinerWahl(id: string): { ladungen: number; nochArmed: boolean } {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.stepPassive(0);
    combat.armResetIfNeeded(combat.indexOf(id), 0);
    const ladungen = combat.resetChargesLeft;
    combat.chooseForReset(alsSpellIndex(combat.indexOf('plate')), 0);
    return { ladungen, nochArmed: combat.resetArmed };
  }

  it('zwei Ladungen, und nach der ersten Wahl steht der Modus noch', () => {
    const { ladungen, nochArmed } = nachEinerWahl('alembic');
    expect(ladungen, 'ALEMBIC gibt nicht zwei Ladungen').toBe(2);
    expect(nochArmed, 'der Auswahlmodus fällt schon nach der ersten Wahl').toBe(true);
  });

  it('REFLUX gibt eine — und fällt nach der ersten Wahl', () => {
    // **Der Kontrast ist die eigentliche Prüfung.** Halbiert man ALEMBIC auf
    // 1, verhält er sich genau wie REFLUX; ohne diesen Fall wäre nicht zu
    // sehen, dass die beiden sich überhaupt unterscheiden sollen.
    const { ladungen, nochArmed } = nachEinerWahl('reflux');
    expect(ladungen).toBe(1);
    expect(nochArmed).toBe(false);
  });

  it('und ein Zauber ohne Ladungen stellt gar nicht scharf', () => {
    const { ladungen, nochArmed } = nachEinerWahl('vapour');
    expect(ladungen).toBe(0);
    expect(nochArmed).toBe(false);
  });
});
