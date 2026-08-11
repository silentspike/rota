/**
 * PINs drei Kontroll-Größen (Issue #478).
 *
 * # Drei von drei ungeschützt
 *
 * `tools/groessen-mutation.py`, alle 116 Paare am 2026-08-11:
 *
 * ```text
 *   control.amountMs  pin  2000 -> 1000              NUR FRISCHE
 *   control.cost      pin  3 -> 1                    NUR FRISCHE
 *   control.effect    pin  makeInterruptible -> …    NUR FRISCHE
 * ```
 *
 * Kein einziges seiner Felder hatte eine Wirkungsprüfung — nur die
 * Vergleichsdatei färbte, und die zieht man nach, ohne zu fragen warum.
 *
 * # Was PIN tut, und warum es messbar ist
 *
 * Er macht den **nächsten** Boss-Eintrag unterbrechbar, auch wenn er es nicht
 * war (`combat.ts`, in `applyControlEffect`). Gemessen gegen `test-caster`,
 * dessen Zeitleiste **nur** `strike` kennt — also nichts, was ohnehin
 * unterbrechbar wäre:
 *
 * ```text
 *   ohne PIN   casting = -1              der Schlag trifft sofort
 *   mit  PIN   casting =  8500           aus dem Schlag wird ein Wirken
 *              landet  = 10500           2000 ms Fenster = control.amountMs
 *              control =  4 -> 1         3 verbraucht = control.cost
 * ```
 *
 * **Alle drei Größen fallen in derselben Messung an.** Das ist kein Zufall:
 * `amountMs` ist die Wirkzeit, die PIN dem Eintrag gibt, `cost` der Preis,
 * `effect` der Schalter davor.
 *
 * # Der Träger ist gemessen, nicht gewählt
 *
 * `test-caster` trägt ausschließlich `strike`. An einem Boss mit
 * `interruptibleCast` wäre der Fall grün, ohne etwas zu zeigen — dort ist der
 * Eintrag ohnehin unterbrechbar, und PIN verpufft. Genau davor warnt der
 * Kommentar an der Wirkstelle: „Ein Fall bliebe GRÜN, weil ja etwas
 * unterbrechbar wurde."
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

interface Lauf {
  /** Kontrollpunkte vor und nach dem Wurf, oder −1 ohne Wurf. */
  readonly controlVor: number;
  readonly controlNach: number;
  /** Wann ein Wirken begann, oder −1. */
  readonly castingAb: number;
  /** Wann es landet — die Differenz ist das Fenster. */
  readonly landetBei: number;
}

/**
 * Sammelt mit BIND Kontrollpunkte und wirft PIN, sobald er sie hat.
 *
 * **BIND ist der Sammler der Schule.** Ohne ihn stünden nie drei Punkte an,
 * und der Wurf käme nie zustande — der Lauf wäre grün und maße nichts.
 */
function lauf(mitPin: boolean): Lauf {
  const boss = bossById('test-caster') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const pin = alsSpellIndex(combat.indexOf('pin'));

  let geworfen = false;
  let controlVor = -1;
  let controlNach = -1;
  let castingAb = -1;
  let landetBei = -1;

  for (let tick = 0; tick < 1200; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (mitPin && !geworfen && combat.control >= 3 && combat.isReady(pin, timeMs)) {
      controlVor = combat.control;
      if (combat.cast(pin, timeMs).result === CAST_OK) {
        geworfen = true;
        controlNach = combat.control;
      }
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }

    if (combat.casting >= 0 && castingAb < 0) {
      castingAb = timeMs;
      landetBei = combat.castLandsAtMs;
    }
  }
  return { controlVor, controlNach, castingAb, landetBei };
}

describe('PIN macht einen Schlag unterbrechbar (control.effect)', () => {
  it('aus einem `strike` wird ein Wirken', () => {
    const mit = lauf(true);
    expect(mit.castingAb, 'PIN erzeugt kein Wirken').toBe(8500);
  });

  it('GEGENPROBE: ohne PIN trifft derselbe Schlag sofort', () => {
    // **Ohne diese Zeile misst der Fall nichts.** Wäre der Eintrag ohnehin
    // unterbrechbar, stünde oben dasselbe Ergebnis — und PIN hätte nichts
    // getan. `test-caster` trägt ausschließlich `strike`, genau deshalb.
    expect(lauf(false).castingAb, 'der Schlag ist auch ohne PIN ein Wirken').toBe(-1);
  });
});

describe('PIN kostet drei Kontrollpunkte (control.cost)', () => {
  it('zieht genau drei ab', () => {
    const mit = lauf(true);
    expect(mit.controlVor, 'es lagen nie Punkte an — der Wurf kam nicht zustande').toBeGreaterThan(
      0,
    );
    expect(mit.controlVor - mit.controlNach, 'PIN kostet nicht drei Punkte').toBe(3);
  });

  it('und der Wurf kommt überhaupt zustande', () => {
    // Die Zusicherung zur Zeile darüber: Ein Lauf, in dem PIN abgewiesen
    // wird, liefert `controlNach === -1` — und `-1 - -1` wäre 0, nicht 3.
    // Ohne diese Prüfung sähe ein nie geworfener Zauber wie ein kostenloser
    // aus.
    expect(lauf(true).controlNach).toBeGreaterThanOrEqual(0);
  });
});

describe('PIN gibt dem Eintrag zwei Sekunden Wirkzeit (control.amountMs)', () => {
  it('das Fenster ist genau 2000 ms breit', () => {
    // `castLandsAtMs = fällig + pinFenster` — die Zahl aus den Daten ist die
    // Zeit, die der Spieler zum Unterbrechen hat. Sie ist damit eine echte
    // Größe und kein Schalter; das war beim Lesen des Codes zunächst unklar,
    // weil die Fallunterscheidung darüber nur `pinFenster > 0` prüft.
    const mit = lauf(true);
    expect(mit.landetBei - mit.castingAb, 'das PIN-Fenster ist nicht 2000 ms').toBe(2000);
  });

  it('und es ist überhaupt eines — die Zusicherung', () => {
    // Ohne sie wäre `landetBei - castingAb === 2000` auch bei zwei −1 grün
    // gewesen: −1 minus −1 ist 0, aber ein anderer Fehlerwert könnte zufällig
    // passen.
    const mit = lauf(true);
    expect(mit.landetBei).toBeGreaterThan(0);
    expect(mit.castingAb).toBeGreaterThan(0);
  });
});
