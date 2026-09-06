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
 * Diese Rechnung beschrieb nur die damalige CANKER/BLIGHT/BLOOM-Folge.
 * Sie ist KEIN Beweis für alle heutigen Kits: Die Produktionsmessung zu #1102
 * erreicht mit SEPSIS und gültigem Cap-Talent acht Stapel OHNE ROT.
 * Siehe docs/STACK-CAPACITY-MEASUREMENT.md für Messweg und Gegenproben.
 *
 * ROT liefert die zweite Quelle, und zwar **ohne GCD**:
 *
 * ```text
 *   rot   dot.decayPerTick 1, tickIntervalMs 1000, durationMs 4000, cd 4000
 * ```
 *
 * ROTs Würfe kosten selbst GCDs. Die frühere Schätzung von rund 1,4 Stapeln/s
 * ist deshalb ersetzt: #1102 misst ohne BLOOM 1,3333 Quellenstapel/s aus ROT
 * und Direktfüllern, mit wiederholtem BLOOM 1,2222/s. Der heutige Deckel von
 * sieben begrenzt die tatsächlich aufgenommenen Stapel zusätzlich.
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
 * `restImFensterNachDemHoechststand` misst, wie viel Fenster nach dem letzten
 * Anstieg übrig blieb. Stünde der Höchststand erst am Ende, wäre das Fenster
 * nicht ausgereizt, sondern abgelaufen.
 *
 * `wuerfeNachDemHoechststand` zählt die Würfe **nach** dem letzten Anstieg. Sie
 * belegt dasselbe von der anderen Seite: Es wurde weitergeworfen, und die Zahl
 * stieg nicht mehr.
 *
 * **Beide zählen Zeit und Würfe, nicht Stapel** — und das ist eine Lehre aus
 * der zweiten Fassung: Dort zählte eine Zeile die stapelnden WÜRFE und verlangte
 * mehr davon, als die Grenze hoch ist. Sie fiel bei sechs, obwohl der Höchststand
 * längst stimmte: ROT liefert **vier** Stapel je Wurf über seine Ticks. Wer
 * Würfe zählt, wo Stapel gemeint sind, misst an einer Stelle, an der das
 * Verhältnis nicht eins zu eins ist.
 *
 * Die erste Fassung dieser Datei ist an `wuerfeNachDemHoechststand` gescheitert,
 * bevor sie eine falsche Zahl melden konnte — und die zweite an einer
 * Trägerzeile, die selbst falsch gebaut war. Beide Male hat der Fall etwas über
 * sich selbst gesagt statt über `decayCap.bonus`, und beide Male war das
 * richtig.
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
  /** Wie viel Fenster nach dem letzten Anstieg noch übrig war, in ms. */
  readonly restImFensterNachDemHoechststand: number;
  /** Gelungene stapelnde Würfe nach dem letzten Anstieg des Höchststands. */
  readonly wuerfeNachDemHoechststand: number;
}

/**
 * Baut mit ROT, CANKER und BLIGHT Stapel auf, wirft PUTRID BLOOM bei vier
 * liegenden und beobachtet den Stand über das ganze Fenster.
 *
 * **ROT hat Vorrang**, weil sein Beitrag nicht am GCD hängt: Ein Wurf kostet
 * einen, liefert aber vier Stapel über seine Ticks. Die enge Gegenprobe ohne
 * ihn bleibt bei sieben; andere Produktionskits sind im Kopf verlinkt.
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
  let hoechstErreichtBei = -1;
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
      if (gelungen && imFenster) wuerfeNachDemHoechststand += 1;
    }

    if (imFenster && combat.decayStacks > hoechststand) {
      hoechststand = combat.decayStacks;
      hoechstErreichtBei = timeMs;
      // Der Zähler beginnt beim NEUEN Höchststand von vorn: Würfe, die
      // stattfanden, während die Zahl noch stieg, belegen nichts über die
      // Grenze — sie zeigen nur, dass es weiterging.
      wuerfeNachDemHoechststand = 0;
    }
  }
  const restImFensterNachDemHoechststand =
    hoechstErreichtBei < 0 ? 0 : wurfBei + FENSTER_MS - hoechstErreichtBei;
  return {
    wurfBei,
    hoechststand,
    restImFensterNachDemHoechststand,
    wuerfeNachDemHoechststand,
  };
}

const GEMESSEN = messen();

describe('der Träger trägt — sonst misst nichts darunter etwas', () => {
  it('PUTRID BLOOM kommt überhaupt zustande', () => {
    // Ein nie geworfener Zauber sähe wie ein wirkungsloser aus, und der
    // Höchststand unten wäre der ungehobene.
    expect(GEMESSEN.wurfBei, 'PUTRID BLOOM kam nicht zustande').toBeGreaterThanOrEqual(0);
  });

  it('der Höchststand fiel lange vor dem Fensterende', () => {
    // Erste Trägerzeile: Stünde er erst am Ende, wäre nicht das Fenster
    // ausgereizt, sondern abgelaufen — dann sagte die Zahl darunter nur, wie
    // weit man in zehn Sekunden kommt.
    //
    // **Die Schwelle ist zwei globale Wartezeiten** (2 × 1500 ms), also
    // mindestens zwei weitere Wurfgelegenheiten. Sie ist aus `GCD_MS`
    // hergeleitet und nicht so lange gedreht worden, bis der Fall grün wurde.
    expect(
      GEMESSEN.restImFensterNachDemHoechststand,
      'der Höchststand fiel erst kurz vor dem Fensterende — dann ist nicht ' +
        'belegt, dass die Grenze gebremst hat und nicht die Zeit',
    ).toBeGreaterThanOrEqual(3000);
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
