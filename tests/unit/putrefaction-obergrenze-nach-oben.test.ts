/**
 * PUTRID BLOOM hebt die Obergrenze um GENAU zwei — auch nach oben (Issue #1094, AC3).
 *
 * # Die Erwartung, hingeschrieben BEVOR ein Lauf stattgefunden hat
 *
 * ```text
 *   decayCap.bonus = 2 (Bestand)   ->   erwartet GRÜN
 *   decayCap.bonus = 3 (Mutation)  ->   erwartet ROT
 * ```
 *
 * Eine Erwartung, die man nach dem Lauf formuliert, ist keine. Trifft sie
 * nicht, ist das das wertvollere Ergebnis — dann zeigt sie einen Denkfehler,
 * statt ihn zu verstecken.
 *
 * # Warum es diese Datei gibt
 *
 * `putrefaction-fenster.test.ts:140` prüft dieselbe Größe und fängt sie nur in
 * **eine** Richtung. Gemessen am 2026-08-22, zehn vitest-Durchgänge, Bezugslauf
 * grün:
 *
 * ```text
 *   decayCap.bonus  2 -> 1 (runter)   1 benannter Prüffall färbt
 *   decayCap.bonus  2 -> 3 (hoch)     0
 * ```
 *
 * Die Null ist der Befund, und die Gegenprobe trägt ihn: Eine negative
 * Kontrolle hätte hier dieselbe Null geliefert wie ein kaputter Messweg.
 *
 * # Warum der bestehende Fall nach oben blind ist
 *
 * ```js
 * expect(MIT_BLOOM.stapelBei12s, 'die Obergrenze wurde nicht um zwei gehoben').toBe(7)
 * ```
 *
 * Bei `bonus` 2 ist die Grenze 5+2 = 7, und der dortige Lauf legt 7 Stapel an.
 * Bei `bonus` 3 wäre die Grenze 8 — der Lauf legt aber weiter nur 7 an, weil
 * ihn die **Zahl der Würfe** begrenzt und nicht die Obergrenze. Er misst den
 * Punkt, an dem beide Größen zufällig zusammenfallen. Nach unten trennen sie
 * sich, nach oben nicht.
 *
 * **Das ist derselbe Befund, den Worker-3 an `decayWard.durationMs` über die
 * Wirksamkeit gefunden hat**, nur über die Deckung gefunden: Die Frage ist
 * nicht die Einheit des Feldes, sondern ob eine Entscheidung genau auf dem
 * Wert liegt.
 *
 * # Was dieser Fall anders macht
 *
 * Er wirft PUTRID BLOOM **so früh wie möglich** statt bei vier Stapeln und
 * misst über das ganze Fenster statt an zwei festen Zeitpunkten. Damit bleiben
 * genug globale Wartezeiten übrig, dass acht Stapel erreichbar *wären*.
 *
 * **Die Trägerzeile ist die eigentliche Arbeit** und heißt hier
 * `wuerfeNachDemHoechststand`: Sie zählt die stapelnden Würfe, die NACH dem
 * erstmaligen Erreichen des Höchststands noch gelingen. Ist sie null, bindet
 * die Wurfzahl und nicht die Grenze — und dann sagt die Zahl darüber nichts,
 * genau wie im bestehenden Fall. Ohne sie hätte diese Datei denselben blinden
 * Fleck, den sie schließen soll.
 */
import { describe, expect, it } from 'vitest';

import { alsSpellIndex, CAST_OK, Combat, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Wie lange das Fenster von PUTRID BLOOM steht (`decayCap.durationMs`). */
const FENSTER_MS = 10_000;

interface Messung {
  /** Wann PUTRID BLOOM fiel, oder −1. */
  readonly wurfBei: number;
  /** Der höchste Stand, der im Fenster je gleichzeitig lag. */
  readonly hoechststand: number;
  /**
   * Gelungene CANKER- und BLIGHT-Würfe nach dem ersten Erreichen des
   * Höchststands, noch im Fenster. Null heißt: Die Wurfzahl bindet.
   */
  readonly wuerfeNachDemHoechststand: number;
}

/**
 * Wirft PUTRID BLOOM, sobald er bereit ist, und danach durchgehend CANKER und
 * BLIGHT. Beobachtet den Stapelstand über das ganze Fenster.
 *
 * `test-onslaught` ist derselbe Träger wie im bestehenden Fall — nicht, weil
 * seine Schläge hier gebraucht würden, sondern damit beide Dateien dieselbe
 * Umgebung messen. Wer sie vergleicht, vergleicht dann wirklich die Fälle.
 */
function messen(): Messung {
  const boss = bossById('test-onslaught') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bloom = alsSpellIndex(combat.indexOf('putrid-bloom'));
  const canker = alsSpellIndex(combat.indexOf('canker'));
  const blight = alsSpellIndex(combat.indexOf('blight'));

  let wurfBei = -1;
  let hoechststand = 0;
  let hoechstErreichtBei = -1;
  let wuerfeNachDemHoechststand = 0;

  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (wurfBei < 0) {
      // So früh wie möglich, nicht bei vier Stapeln: Jede Wartezeit vorher
      // kostet Fenster, und das Fenster ist hier die knappe Größe.
      if (combat.isReady(bloom, timeMs) && combat.cast(bloom, timeMs).result === CAST_OK) {
        wurfBei = timeMs;
      }
    } else if (timeMs <= wurfBei + FENSTER_MS) {
      let gelungen = false;
      if (combat.isReady(canker, timeMs)) {
        gelungen = combat.cast(canker, timeMs).result === CAST_OK;
      } else if (combat.isReady(blight, timeMs)) {
        gelungen = combat.cast(blight, timeMs).result === CAST_OK;
      }
      if (gelungen && hoechstErreichtBei >= 0) wuerfeNachDemHoechststand += 1;
    }

    if (wurfBei >= 0 && timeMs <= wurfBei + FENSTER_MS && combat.decayStacks > hoechststand) {
      hoechststand = combat.decayStacks;
      hoechstErreichtBei = timeMs;
      // Der Zähler beginnt beim NEUEN Höchststand von vorn: Würfe, die
      // stattfanden, während die Zahl noch stieg, belegen nichts über die
      // Grenze — sie zeigen nur, dass es weiterging.
      wuerfeNachDemHoechststand = 0;
    }
  }
  return { wurfBei, hoechststand, wuerfeNachDemHoechststand };
}

const GEMESSEN = messen();

describe('der Träger trägt — sonst misst nichts darunter etwas', () => {
  it('PUTRID BLOOM kommt überhaupt zustande', () => {
    // Ein nie geworfener Zauber sähe wie ein wirkungsloser aus, und der
    // Höchststand unten wäre der ungehobene.
    expect(GEMESSEN.wurfBei, 'PUTRID BLOOM kam nicht zustande').toBeGreaterThanOrEqual(0);
  });

  it('die Grenze bindet, nicht die Wurfzahl', () => {
    // **Die Zeile, an der alles hängt.** Nach dem Höchststand wurde
    // weitergeworfen, und die Zahl stieg nicht mehr. Wäre sie null, hätte der
    // Lauf schlicht aufgehört zu werfen — und dann sagte der Höchststand
    // nichts über die Obergrenze, genau wie im Fall, den diese Datei ersetzt.
    expect(
      GEMESSEN.wuerfeNachDemHoechststand,
      'nach dem Höchststand gelang kein stapelnder Wurf mehr — dann bindet die ' +
        'Wurfzahl und nicht die Grenze, und der Fall darunter misst den Zufall',
    ).toBeGreaterThan(0);
  });
});

describe('PUTRID BLOOM hebt die Obergrenze um genau zwei (decayCap.bonus)', () => {
  it('sieben, nicht acht — die Anhebung ist zwei und nicht drei', () => {
    // Diese Zusicherung ist der Grund für die ganze Datei: Sie wird bei
    // `bonus` 3 rot. Der bestehende Fall in putrefaction-fenster.test.ts
    // bliebe dort grün.
    expect(
      GEMESSEN.hoechststand,
      'die Obergrenze liegt nicht bei sieben — bei mehr ist `decayCap.bonus` ' +
        'gewachsen, bei weniger gesenkt',
    ).toBe(7);
  });

  it('und sie ist gegen die Grundgrenze gerechnet, nicht absolut gesetzt', () => {
    // Sonst wäre die Sieben oben mit einer geänderten Grundgrenze vereinbar:
    // MAX_DECAY_STACKS 4 plus bonus 3 ergäbe ebenfalls sieben.
    expect(MAX_DECAY_STACKS, 'die Grundgrenze hat sich geändert').toBe(5);
    expect(
      GEMESSEN.hoechststand - MAX_DECAY_STACKS,
      'der Abstand zur Grundgrenze ist nicht zwei',
    ).toBe(2);
  });
});
