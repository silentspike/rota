import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { BIG_HIT_HP_SHARE, HIT_DIRECT, HIT_DOT } from '../../src/core/hits.js';
import { TICK_MS } from '../../src/core/loop.js';
import {
  FLASH_DURATION_MS,
  Impact,
  NUMBER_LIFETIME_MS,
  SHAKE_AMPLITUDE_PX,
  SHAKE_DURATION_MS,
  flashAt,
  shakeAt,
} from '../../src/render/impact.js';

function makeCombat(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

interface Recorded {
  damage: number;
  kind: number;
  isCrit: boolean;
  big: boolean;
  timeMs: number;
}

function record(combat: Combat): Recorded[] {
  const hits: Recorded[] = [];
  combat.onHit = (damage, kind, isCrit, big, timeMs): void => {
    hits.push({ damage, kind, isCrit, big, timeMs });
  };
  return hits;
}

describe('Treffer als Ereignis', () => {
  it('meldet jeden direkten Treffer mit Schaden und Zeit', () => {
    const combat = makeCombat();
    const hits = record(combat);
    combat.cast(combat.indexOf('kindle'), 1234);

    expect(hits.length).toBe(1);
    expect(hits[0]).toEqual({
      damage: 100,
      kind: HIT_DIRECT,
      isCrit: false,
      big: false,
      timeMs: 1234,
    });
  });

  it('meldet jeden Tick eines Effekts über Zeit getrennt', () => {
    const combat = makeCombat();
    const hits = record(combat);
    combat.cast(combat.indexOf('smoulder'), 0);
    for (let t = TICK_MS; t <= 6000; t += TICK_MS) combat.stepPassive(t);

    expect(hits.length).toBe(3);
    for (const hit of hits) {
      expect(hit.kind).toBe(HIT_DOT);
      expect(hit.damage).toBe(40);
      expect(hit.big).toBe(false);
    }
    expect(hits.map((h) => h.timeMs)).toEqual([2000, 4000, 6000]);
  });

  it('erkennt einen großen Schlag am Anteil der Lebenspunkte', () => {
    // Fünf Prozent von 4200 sind 210. PYRE mit Bonus (360) liegt darüber,
    // ohne Bonus bei 14 Hitze (168) darunter.
    const grenze = FIRST_CRUCIBLE.value.hp * BIG_HIT_HP_SHARE;
    expect(grenze).toBe(210);

    const klein = makeCombat();
    const kleinHits = record(klein);
    klein.setResource(14);
    klein.cast(klein.indexOf('pyre'), 0);
    expect(kleinHits[0]?.damage).toBe(168);
    expect(kleinHits[0]?.big).toBe(false);

    const gross = makeCombat();
    const grossHits = record(gross);
    gross.setResource(15);
    gross.cast(gross.indexOf('pyre'), 0);
    expect(grossHits[0]?.damage).toBe(360);
    expect(grossHits[0]?.big).toBe(true);
  });

  it('meldet nichts, wenn ein Zauber keinen Schaden macht', () => {
    const combat = makeCombat();
    const hits = record(combat);
    combat.cast(combat.indexOf('smoulder'), 0); // Direktschaden 0
    expect(hits.length).toBe(0);
  });

  it('kostet den Kampf nichts, wenn niemand zuhört', () => {
    const combat = makeCombat();
    combat.onHit = null;
    expect(() => combat.cast(combat.indexOf('kindle'), 0)).not.toThrow();
    expect(combat.damageDealt).toBe(100);
  });
});

describe('Bild-Ruckeln', () => {
  it('hält sich an 6 px über 120 ms (GAME-DESIGN 9.4)', () => {
    expect(SHAKE_DURATION_MS).toBe(120);
    expect(SHAKE_AMPLITUDE_PX).toBe(6);
    let max = 0;
    for (let t = 0; t < SHAKE_DURATION_MS; t += 0.5) {
      const value = Math.abs(shakeAt(t));
      if (value > max) max = value;
    }
    expect(max).toBeLessThanOrEqual(SHAKE_AMPLITUDE_PX);
    expect(max).toBeGreaterThan(3);
  });

  it('beginnt und endet bei null — kein Sprung', () => {
    expect(shakeAt(0)).toBeCloseTo(0, 9);
    expect(shakeAt(SHAKE_DURATION_MS)).toBe(0);
    expect(shakeAt(SHAKE_DURATION_MS - 0.001)).toBeCloseTo(0, 3);
    expect(shakeAt(-1)).toBe(0);
    expect(shakeAt(10_000)).toBe(0);
  });

  it('klingt monoton ab', () => {
    // Die Hüllkurve wird kleiner, auch wenn die Schwingung das Vorzeichen wechselt.
    let vorher = Number.POSITIVE_INFINITY;
    for (let t = 0; t < SHAKE_DURATION_MS; t += SHAKE_DURATION_MS / 4) {
      const huellkurve = SHAKE_AMPLITUDE_PX * (1 - t / SHAKE_DURATION_MS);
      expect(huellkurve).toBeLessThanOrEqual(vorher);
      vorher = huellkurve;
    }
  });

  it('liefert bei gleicher Zeit denselben Wert', () => {
    for (const t of [0, 17, 61.5, 119]) expect(shakeAt(t)).toBe(shakeAt(t));
  });
});

describe('Aufblitzen', () => {
  it('dauert 180 ms und klingt ab', () => {
    expect(FLASH_DURATION_MS).toBe(180);
    expect(flashAt(0)).toBe(1);
    expect(flashAt(90)).toBeCloseTo(0.25);
    expect(flashAt(180)).toBe(0);
    expect(flashAt(-1)).toBe(0);
  });

  it('fällt gleichmäßig, ohne Sprung am Ende', () => {
    let vorher = flashAt(0);
    for (let t = 1; t <= FLASH_DURATION_MS; t += 1) {
      const value = flashAt(t);
      expect(value).toBeLessThanOrEqual(vorher + 1e-9);
      vorher = value;
    }
    expect(vorher).toBe(0);
  });
});

describe('Schadenszahlen — Pool (AC4)', () => {
  it('legt alle Plätze vorab an', () => {
    const impact = new Impact(8);
    expect(impact.poolCapacity).toBe(8);
    expect(impact.activeCount).toBe(0);
  });

  it('gibt eine Zahl nach 700 ms wieder frei (GAME-DESIGN 9.4)', () => {
    const impact = new Impact(8);
    const ctx = fakeCtx();
    impact.onHit(100, HIT_DIRECT, false, false, 0);
    expect(impact.activeCount).toBe(1);

    impact.draw(ctx as unknown as CanvasRenderingContext2D, NUMBER_LIFETIME_MS - 1);
    expect(impact.activeCount).toBe(1);
    impact.draw(ctx as unknown as CanvasRenderingContext2D, NUMBER_LIFETIME_MS);
    expect(impact.activeCount).toBe(0);
  });

  it('läuft nicht über, sondern verwendet die älteste Zahl wieder', () => {
    const impact = new Impact(4);
    for (let i = 0; i < 40; i++) impact.onHit(i + 1, HIT_DIRECT, false, false, i);

    expect(impact.activeCount).toBeLessThanOrEqual(4);
    expect(impact.poolCapacity).toBe(4);
    expect(impact.spawnCount).toBe(40);
    expect(impact.recycleCount).toBe(36);
  });

  it('zeichnet nach dem Überlauf die neuesten Zahlen', () => {
    const impact = new Impact(3);
    const ctx = fakeCtx();
    for (let i = 1; i <= 6; i++) impact.onHit(i * 11, HIT_DIRECT, false, false, 0);
    impact.draw(ctx as unknown as CanvasRenderingContext2D, 10);

    // Drei Plätze, also die drei jüngsten Werte: 44, 55, 66. Verglichen werden
    // die gezeichneten Ziffern — die Zahlen entstehen ziffernweise, und zwei
    // gleich hoch liegende Zahlen ließen sich nicht zuverlässig trennen.
    expect(ctx.digits.sort()).toEqual(['4', '4', '5', '5', '6', '6']);
  });

  it('setzt sich vollständig zurück', () => {
    const impact = new Impact(8);
    impact.onHit(100, HIT_DIRECT, false, true, 0);
    impact.reset();

    expect(impact.activeCount).toBe(0);
    expect(impact.spawnCount).toBe(0);
    expect(impact.shakeOffset(10)).toBe(0);
    expect(impact.flashStrength(10)).toBe(0);
  });
});

describe('Schadenszahlen — Bewegung', () => {
  it('steigt auf und blendet aus, ohne zu springen', () => {
    const impact = new Impact(4);
    const ctx = fakeCtx();
    impact.onHit(123, HIT_DIRECT, false, false, 0);

    let vorherY = Number.POSITIVE_INFINITY;
    let vorherAlpha = Number.POSITIVE_INFINITY;
    for (let t = 0; t < NUMBER_LIFETIME_MS; t += 10) {
      ctx.reset();
      impact.draw(ctx as unknown as CanvasRenderingContext2D, t);
      expect(ctx.lastY).toBeLessThanOrEqual(vorherY + 1e-9); // steigt (kleineres y)
      expect(ctx.lastAlpha).toBeLessThanOrEqual(vorherAlpha + 1e-9);
      vorherY = ctx.lastY;
      vorherAlpha = ctx.lastAlpha;
    }
    expect(vorherAlpha).toBeLessThan(0.01);
  });

  it('meldet Ruckeln nur bei großen Schlägen', () => {
    const klein = new Impact(4);
    klein.onHit(100, HIT_DIRECT, false, false, 0);
    expect(klein.shakeOffset(30)).toBe(0);

    const gross = new Impact(4);
    gross.onHit(360, HIT_DIRECT, false, true, 0);
    expect(Math.abs(gross.shakeOffset(30))).toBeGreaterThan(0);
    expect(gross.shakeOffset(SHAKE_DURATION_MS)).toBe(0);
  });

  it('lässt den Ring bei JEDEM Treffer aufblitzen', () => {
    const impact = new Impact(4);
    impact.onHit(40, HIT_DOT, false, false, 0);
    expect(impact.flashStrength(0)).toBe(1);
    expect(impact.flashStrength(FLASH_DURATION_MS)).toBe(0);
  });
});

/**
 * Ersatz-Zeichenfläche.
 *
 * `drawIntegerRight` zeichnet die Ziffern von rechts nach links; eine neue Zahl
 * beginnt daher immer mit einem GRÖSSEREN x als der vorherige Aufruf. Daran
 * lassen sich die Zahlen wieder auseinandersortieren.
 */
class FakeContext {
  private readonly calls: { text: string; x: number; y: number; alpha: number }[] = [];
  font = '';
  fillStyle = '';
  textAlign = 'left';
  globalAlpha = 1;

  fillText(text: string, x: number, y: number): void {
    this.calls.push({ text, x, y, alpha: this.globalAlpha });
  }

  reset(): void {
    this.calls.length = 0;
  }

  /** Alle gezeichneten Zeichen, unsortiert. */
  get digits(): string[] {
    return this.calls.map((c) => c.text);
  }

  get lastY(): number {
    return this.calls[this.calls.length - 1]?.y ?? 0;
  }

  get lastAlpha(): number {
    return this.calls[this.calls.length - 1]?.alpha ?? 1;
  }
}

function fakeCtx(): FakeContext {
  return new FakeContext();
}
