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
 * Eine Erwartung, die man nach dem Lauf formuliert, ist keine.
 *
 * **Die erste Fassung dieser Datei hat sie verfehlt, und das war das wertvollere
 * Ergebnis** — sie steht deshalb hier mitsamt der Rechnung, die sie stützt.
 *
 * # Warum die erste Fassung nicht trug: acht Stapel waren unerreichbar
 *
 * ```text
 *   DECAY_STACK_DURATION_MS   12000   (combat.ts)
 *   GCD_MS                     1500   (gcd.ts)
 * ```
 *
 * 12000 / 1500 = **acht** Stapel liegen maximal gleichzeitig, wenn man
 * ununterbrochen CANKER und BLIGHT wirft. PUTRID BLOOM kostet selbst einen
 * GCD — bleiben **sieben**.
 *
 * Damit ist die Acht ohne eine zweite Stapelquelle strukturell unerreichbar,
 * egal wann man wirft und wie lange man misst. Der bestehende Fall
 * `putrefaction-fenster.test.ts:140` bleibt bei `bonus` 3 aus genau diesem
 * Grund grün: Es ist keine Eigenart seiner Messpunkte, sondern eine Grenze des
 * Kits.
 *
 * ROT liefert die zweite Quelle, und zwar **ohne GCD**:
 *
 * ```text
 *   rot   dot.decayPerTick 1, tickIntervalMs 1000, durationMs 4000, cd 4000
 * ```
 *
 * Ein Stapel je Sekunde aus den DoT-Ticks, dauerhaft haltbar (Laufzeit gleich
 * Abklingzeit). Die Rate steigt von rund 0,67 auf rund 1,4 Stapel je Sekunde —
 * acht gleichzeitig liegende sind dann erreichbar statt ausgeschlossen.
 *
 * # Warum es diese Datei überhaupt gibt
 *
 * `putrefaction-fenster.test.ts:140` prüft dieselbe Größe und fängt sie nur in
 * **eine** Richtung. Gemessen am 2026-08-22, zehn vitest-Durchgänge:
 *
 * ```text
 *   decayCap.bonus  2 -> 1 (runter)   1 benannter Prüffall färbt
 *   decayCap.bonus  2 -> 3 (hoch)     0
 * ```
 *
 * Die Null ist der Befund, und die Gegenprobe trägt ihn: Eine negative
 * Kontrolle hätte hier dieselbe Null geliefert wie ein kaputter Messweg.
 *
 * # Die zwei Trägerzeilen, und warum es zwei sind
 *
 * `stapelndeWuerfeImFenster` zählt, wie viele stapelnde Würfe überhaupt
 * gelangen. Ist die Zahl nicht größer als die erwartete Grenze, bindet die
 * Wurfzahl — dann sagt der Höchststand nichts über `decayCap.bonus`.
 *
 * `wuerfeNachDemHoechststand` zählt die Würfe **nach** dem letzten Anstieg. Sie
 * belegt dasselbe von der anderen Seite: Es wurde weitergeworfen, und die Zahl
 * stieg nicht mehr.
 *
 * Die erste kann trügen, wenn die Würfe früh liegen und die Stapel verfallen;
 * die zweite, wenn der Höchststand erst am Fensterende fällt. **Zusammen decken
 * sie beide Fälle ab** — und die erste Fassung dieser Datei ist an der zweiten
 * gescheitert, bevor sie eine falsche Zahl melden konnte.
 */
import { describe, expect, it } from 'vitest';

import { alsSpellIndex, CAST_OK, Combat, MAX_DECAY_STACKS } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Wie lange das Fenster von PUTRID BLOOM steht (`decayCap.durationMs`). */
const FENSTER_MS = 10_000;

/**
 * Bei wie vielen liegenden Stapeln PUTRID BLOOM fällt.
 *
 * **Vier, nicht null und nicht fünf** — dieselbe Wahl wie im bestehenden Fall,
 * und aus zwei Gründen: Bei fünf wäre die alte Obergrenze schon erreicht und
 * der Zauber käme zu spät; bei null verschenkt man die Zeit zum Stapeln, die
 * vor dem Fenster liegt. Die erste Fassung dieser Datei warf bei null und kam
 * deshalb auf sechs.
 */
const WERFEN_BEI = 4;

interface Messung {
  /** Wann PUTRID BLOOM fiel, oder −1. */
  readonly wurfBei: number;
  /** Der höchste Stand, der im Fenster je gleichzeitig lag. */
  readonly hoechststand: number;
  /** Gelungene stapelnde Würfe im Fenster insgesamt. */
  readonly stapelndeWuerfeImFenster: number;
  /** Davon die nach dem letzten Anstieg des Höchststands. */
  readonly wuerfeNachDemHoechststand: number;
}

/**
 * Baut mit ROT, CANKER und BLIGHT Stapel auf, wirft PUTRID BLOOM bei vier
 * liegenden und beobachtet den Stand über das ganze Fenster.
 *
 * **ROT hat Vorrang**, weil sein Beitrag nicht am GCD hängt: Ein Wurf kostet
 * einen, liefert aber vier Stapel über seine Ticks. Ohne ihn ist die Acht
 * unerreichbar, siehe Kopf.
 *
 * `test-onslaught` ist derselbe Träger wie im bestehenden Fall — nicht, weil
 * seine Schläge hier gebraucht würden, sondern damit beide Dateien dieselbe
 * Umgebung messen. Wer sie vergleicht, vergleicht dann wirklich die Fälle.
 */
function messen(): Messung {
  const boss = bossById('test-onslaught') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bloom = alsSpellIndex(combat.indexOf('putrid-bloom'));
  const rot = alsSpellIndex(combat.indexOf('rot'));
  const canker = alsSpellIndex(combat.indexOf('canker'));
  const blight = alsSpellIndex(combat.indexOf('blight'));

  let wurfBei = -1;
  let hoechststand = 0;
  let stapelndeWuerfeImFenster = 0;
  let wuerfeNachDemHoechststand = 0;

  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    const imFenster = wurfBei >= 0 && timeMs <= wurfBei + FENSTER_MS;

    if (wurfBei < 0 && combat.decayStacks >= WERFEN_BEI && combat.isReady(bloom, timeMs)) {
      if (combat.cast(bloom, timeMs).result === CAST_OK) wurfBei = timeMs;
    } else {
      let gelungen = false;
      for (const zauber of [rot, canker, blight]) {
        if (combat.isReady(zauber, timeMs)) {
          gelungen = combat.cast(zauber, timeMs).result === CAST_OK;
          break;
        }
      }
      if (gelungen && imFenster) {
        stapelndeWuerfeImFenster += 1;
        wuerfeNachDemHoechststand += 1;
      }
    }

    if (imFenster && combat.decayStacks > hoechststand) {
      hoechststand = combat.decayStacks;
      // Der Zähler beginnt beim NEUEN Höchststand von vorn: Würfe, die
      // stattfanden, während die Zahl noch stieg, belegen nichts über die
      // Grenze — sie zeigen nur, dass es weiterging.
      wuerfeNachDemHoechststand = 0;
    }
  }
  return { wurfBei, hoechststand, stapelndeWuerfeImFenster, wuerfeNachDemHoechststand };
}

const GEMESSEN = messen();

describe('der Träger trägt — sonst misst nichts darunter etwas', () => {
  it('PUTRID BLOOM kommt überhaupt zustande', () => {
    // Ein nie geworfener Zauber sähe wie ein wirkungsloser aus, und der
    // Höchststand unten wäre der ungehobene.
    expect(GEMESSEN.wurfBei, 'PUTRID BLOOM kam nicht zustande').toBeGreaterThanOrEqual(0);
  });

  it('es wurde öfter gestapelt, als die Grenze zulässt', () => {
    // Erste Trägerzeile: Wären es weniger Würfe als die Grenze, käme der
    // Höchststand aus der Wurfzahl und nicht aus `decayCap.bonus`.
    expect(
      GEMESSEN.stapelndeWuerfeImFenster,
      'im Fenster gelangen nicht mehr stapelnde Würfe als die Grenze hoch ist — ' +
        'dann bindet die Wurfzahl, und die Zahl darunter misst den Zufall',
    ).toBeGreaterThan(MAX_DECAY_STACKS + 3);
  });

  it('die Grenze bindet, nicht die Wurfzahl', () => {
    // Zweite Trägerzeile, von der anderen Seite: Nach dem letzten Anstieg
    // wurde weitergeworfen, und die Zahl stieg nicht mehr.
    expect(
      GEMESSEN.wuerfeNachDemHoechststand,
      'nach dem Höchststand gelang kein stapelnder Wurf mehr — dann ist nicht ' +
        'belegt, dass die Grenze und nicht das Ende des Fensters gebremst hat',
    ).toBeGreaterThan(0);
  });
});

describe('PUTRID BLOOM hebt die Obergrenze um genau zwei (decayCap.bonus)', () => {
  it('sieben, nicht acht — die Anhebung ist zwei und nicht drei', () => {
    // Diese Zusicherung ist der Grund für die ganze Datei: Sie wird bei
    // `bonus` 3 rot. Der bestehende Fall in putrefaction-fenster.test.ts
    // bliebe dort grün, weil ihm die zweite Stapelquelle fehlt.
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
