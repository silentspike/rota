import { describe, expect, it } from 'vitest';
import {
  QUALITY_COMMON,
  QUALITY_COUNT,
  QUALITY_EPIC,
  QUALITY_LEGENDARY,
  QUALITY_RARE,
} from '../../src/gear/budget.js';
import {
  LootMoment,
  labelFor,
  presentationFor,
  presentationsAreDistinct,
} from '../../src/loot/moment.js';
import {
  APPROXIMATION_CAVEAT,
  COMPARE_BETTER,
  COMPARE_EQUAL,
  COMPARE_WORSE,
  compareItems,
  isMarkedAsApproximation,
} from '../../src/loot/compare.js';

describe('Vier Qualitäten, vier Inszenierungen (AC3)', () => {
  it('unterscheidet JEDES Paar, nicht nur das erste und letzte', () => {
    expect(presentationsAreDistinct()).toBe(true);
    // Und ausgeschrieben, damit die Meldung sagt, welches Paar klemmt.
    for (let a = 0; a < QUALITY_COUNT; a++) {
      for (let b = a + 1; b < QUALITY_COUNT; b++) {
        expect(presentationFor(a).className, `${a} gegen ${b}`).not.toBe(
          presentationFor(b).className,
        );
        expect(presentationFor(a).durationMs, `${a} gegen ${b}`).not.toBe(
          presentationFor(b).durationMs,
        );
      }
    }
  });

  it('wird mit jeder Stufe deutlicher', () => {
    // Als Ungleichung, nicht gegen Zahlen: Die Werte sind vorläufig, die
    // Ordnung ist es nicht.
    expect(presentationFor(QUALITY_RARE).durationMs).toBeGreaterThan(
      presentationFor(QUALITY_COMMON).durationMs,
    );
    expect(presentationFor(QUALITY_EPIC).durationMs).toBeGreaterThan(
      presentationFor(QUALITY_RARE).durationMs,
    );
    expect(presentationFor(QUALITY_LEGENDARY).durationMs).toBeGreaterThan(
      presentationFor(QUALITY_EPIC).durationMs,
    );
  });

  it('hält NUR beim Legendary an', () => {
    // „99 von 100 Siegen laufen schnell durch, der eine Moment brennt sich
    // ein." Hielte auch Epic an, wäre der Legendary-Moment gewöhnlich.
    expect(presentationFor(QUALITY_LEGENDARY).halts).toBe(true);
    expect(presentationFor(QUALITY_EPIC).halts).toBe(false);
    expect(presentationFor(QUALITY_RARE).halts).toBe(false);
    expect(presentationFor(QUALITY_COMMON).halts).toBe(false);
  });

  it('lässt Common beiläufig — der Fall, der kurz bleiben MUSS', () => {
    // Der Vergleichsfall zur Staffelung: Wäre Common nicht kurz, zerstörte
    // die Inszenierung genau das, wofür sie da ist.
    const common = presentationFor(QUALITY_COMMON);
    expect(common.dim).toBe(0);
    expect(common.sound).toBe('');
    expect(common.durationMs).toBeLessThan(presentationFor(QUALITY_LEGENDARY).durationMs / 4);
  });

  it('schreibt die Namen englisch (Design-Gesetz 5)', () => {
    expect(labelFor(QUALITY_LEGENDARY)).toBe('LEGENDARY');
    expect(labelFor(QUALITY_COMMON)).toBe('COMMON');
  });
});

describe('Eine Taste überspringt alles (AC4)', () => {
  it('bricht auch den Legendary-Moment ab', () => {
    // „Vollständig überspringbar" — auch den. Ein Moment, den man nicht
    // abbrechen kann, ist beim zweiten Mal eine Zumutung.
    const moment = new LootMoment();
    moment.show(QUALITY_LEGENDARY, 0);
    expect(moment.isRunning).toBe(true);
    expect(moment.halts).toBe(true);

    expect(moment.skip()).toBe(true);
    expect(moment.isRunning).toBe(false);
    expect(moment.halts).toBe(false);
    expect(moment.isActiveAt(10)).toBe(false);
  });

  it('überspringt jede Stufe, nicht nur die kurzen', () => {
    for (let quality = 0; quality < QUALITY_COUNT; quality++) {
      const moment = new LootMoment();
      moment.show(quality, 0);
      expect(moment.skip(), `Qualität ${quality}`).toBe(true);
      expect(moment.isRunning, `Qualität ${quality}`).toBe(false);
    }
  });

  it('zählt einen Druck ins Leere nicht als Überspringen', () => {
    const moment = new LootMoment();
    expect(moment.skip()).toBe(false);
    expect(moment.skippedTotal).toBe(0);
  });

  it('läuft ohne Taste von selbst ab — der Vergleichsfall', () => {
    // Ohne diesen Lauf belegte AC4 nur, dass die Taste etwas tut, nicht dass
    // sie etwas ABKÜRZT.
    const moment = new LootMoment();
    const dauer = presentationFor(QUALITY_EPIC).durationMs;
    moment.show(QUALITY_EPIC, 0);

    expect(moment.isActiveAt(dauer - 1)).toBe(true);
    moment.step(dauer - 1);
    expect(moment.isRunning).toBe(true);

    moment.step(dauer);
    expect(moment.isRunning).toBe(false);
    expect(moment.skippedTotal).toBe(0);
  });
});

describe('Der Vergleich nennt eine Näherung (AC5)', () => {
  it('kennzeichnet sie an ZWEI Merkmalen — Zeichen und Wort', () => {
    // Ein Zeichen allein wird übersehen, ein Wort allein überlesen. Dieselbe
    // Überlegung wie bei additiv/multiplikativ (#114).
    const besser = compareItems(1200, 1000);
    expect(besser.text).toBe('~+20%');
    expect(besser.caveat).toBe(APPROXIMATION_CAVEAT);
    expect(isMarkedAsApproximation(besser)).toBe(true);
  });

  it('trägt das Näherungszeichen AUCH bei glatten Zahlen und bei null', () => {
    // Die Ausnahme wäre die Stelle, an der jemand die Angabe für exakt hielte.
    expect(compareItems(1000, 1000).text).toBe('~0%');
    expect(compareItems(2000, 1000).text).toBe('~+100%');
    expect(compareItems(500, 1000).text).toBe('~-50%');
    for (const c of [compareItems(1000, 1000), compareItems(500, 1000)]) {
      expect(isMarkedAsApproximation(c)).toBe(true);
    }
  });

  it('sagt, ob der Fund besser ist', () => {
    expect(compareItems(1200, 1000).verdict).toBe(COMPARE_BETTER);
    expect(compareItems(1000, 1000).verdict).toBe(COMPARE_EQUAL);
    expect(compareItems(800, 1000).verdict).toBe(COMPARE_WORSE);
  });

  it('bleibt bei unbrauchbaren Zahlen bei null, statt etwas zu erfinden', () => {
    for (const [a, b] of [
      [1000, 0],
      [Number.NaN, 1000],
      [1000, Number.POSITIVE_INFINITY],
    ]) {
      const c = compareItems(a as number, b as number);
      expect(c.damagePercent).toBe(0);
      expect(isMarkedAsApproximation(c)).toBe(true);
    }
  });
});
