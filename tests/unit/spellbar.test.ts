import { describe, expect, it } from 'vitest';
import {
  BLOCK_LEFT,
  BLOCK_TOP,
  BLOCK_WIDTH,
  SLOT_GAP,
  SLOT_SIZE,
  SPACE_HEIGHT,
  drawSweep,
  slotRect,
  sweepHandAngle,
  type Rect,
} from '../../src/render/spellbar.js';
import { KEYS_PER_LAYER, SLOT_COUNT, SPACE_POSITION, slotOf } from '../../src/input/keys.js';

function rectOf(slot: number): Rect {
  const out: Rect = { x: 0, y: 0, width: 0, height: 0 };
  slotRect(slot, out);
  return out;
}

describe('Zauberleiste — Anordnung', () => {
  it('bildet die Tastatur nach: QWER oben, ASDF mitte, YXCV unten', () => {
    const q = rectOf(0);
    const w = rectOf(1);
    const a = rectOf(4);
    const z = rectOf(8);

    // Q W E R nebeneinander in derselben Zeile
    expect(w.y).toBe(q.y);
    expect(w.x - q.x).toBe(SLOT_SIZE + SLOT_GAP);
    // A S D F eine Zeile darunter, wieder ganz links beginnend
    expect(a.x).toBe(q.x);
    expect(a.y - q.y).toBe(SLOT_SIZE + SLOT_GAP);
    // Y X C V noch eine Zeile darunter
    expect(z.x).toBe(q.x);
    expect(z.y - a.y).toBe(SLOT_SIZE + SLOT_GAP);
  });

  it('gibt der Leertaste eine eigene Zeile über die volle Breite', () => {
    const space = rectOf(SPACE_POSITION);
    expect(space.x).toBe(BLOCK_LEFT[0]);
    expect(space.width).toBe(BLOCK_WIDTH);
    expect(space.height).toBe(SPACE_HEIGHT);
    expect(space.y).toBeGreaterThan(rectOf(8).y);
  });

  it('legt die Umschalt-Ebene als zweiten Block daneben', () => {
    const direct = rectOf(0);
    const shift = rectOf(KEYS_PER_LAYER);
    expect(shift.y).toBe(direct.y);
    expect(shift.x).toBe(BLOCK_LEFT[1]);
    expect(shift.x).toBeGreaterThan(direct.x + BLOCK_WIDTH);
  });

  it('lässt die Blöcke einander nicht überlappen', () => {
    const rightEdgeOfDirect = (BLOCK_LEFT[0] as number) + BLOCK_WIDTH;
    expect(BLOCK_LEFT[1] as number).toBeGreaterThan(rightEdgeOfDirect);
  });

  it('legt die Leiste nach unten links', () => {
    // GAME-DESIGN 9.1: Zauberleiste unten links.
    expect(BLOCK_LEFT[0] as number).toBeLessThan(200);
    expect(BLOCK_TOP).toBeGreaterThan(540);
  });

  it('gibt jedem der 26 Plätze eine eigene Fläche', () => {
    const seen = new Set<string>();
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const r = rectOf(slot);
      const key = `${r.x}/${r.y}`;
      expect(seen.has(key)).toBe(false);
      seen.add(key);
      expect(r.width).toBeGreaterThan(0);
      expect(r.height).toBeGreaterThan(0);
    }
    expect(seen.size).toBe(SLOT_COUNT);
  });

  it('hält die Lage jedes Platzes fest, unabhängig von der Belegung (AC5)', () => {
    // Die Lage ist eine reine Funktion des Platzes. Ein später dazukommender
    // Zauber kann bestehende deshalb gar nicht verschieben — das ist die Auflage
    // aus GAME-DESIGN 9.1 zum Muskelgedächtnis.
    const before: Rect[] = [];
    for (let slot = 0; slot < SLOT_COUNT; slot++) before.push(rectOf(slot));
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      expect(rectOf(slot)).toEqual(before[slot]);
    }
  });

  it('setzt die Umschalt-Ebene aus denselben Stellen zusammen', () => {
    for (let position = 0; position < KEYS_PER_LAYER; position++) {
      const direct = rectOf(slotOf(0, position));
      const shift = rectOf(slotOf(1, position));
      expect(shift.y).toBe(direct.y);
      expect(shift.width).toBe(direct.width);
      expect(shift.height).toBe(direct.height);
      expect(shift.x - direct.x).toBe((BLOCK_LEFT[1] as number) - (BLOCK_LEFT[0] as number));
    }
  });
});

describe('Wischblende — Winkel', () => {
  it('beginnt bei voller Abklingzeit auf zwölf Uhr', () => {
    // -PI/2 ist zwölf Uhr: Der Schleier bedeckt von dort aus den vollen Kreis.
    expect(sweepHandAngle(1)).toBeCloseTo(-Math.PI / 2);
  });

  it('läuft im Uhrzeigersinn und endet nach genau einer Umdrehung', () => {
    expect(sweepHandAngle(0.75)).toBeCloseTo(-Math.PI / 2 + Math.PI / 2);
    expect(sweepHandAngle(0.5)).toBeCloseTo(-Math.PI / 2 + Math.PI);
    expect(sweepHandAngle(0.25)).toBeCloseTo(-Math.PI / 2 + 1.5 * Math.PI);
    expect(sweepHandAngle(0)).toBeCloseTo(-Math.PI / 2 + 2 * Math.PI);
  });

  it('bewegt sich gleichmäßig — gleiche Zeitschritte, gleiche Winkelschritte', () => {
    // Kein Springen: Der Winkel ist eine lineare Funktion der Restzeit.
    let previous = sweepHandAngle(1);
    const step = sweepHandAngle(0.9) - previous;
    for (let share = 0.9; share >= 0; share -= 0.1) {
      const angle = sweepHandAngle(share);
      expect(angle - previous).toBeCloseTo(step, 9);
      previous = angle;
    }
  });
});

/** Ersatz-Zeichenfläche, die nur festhält, was aufgerufen wurde. */
class FakeContext {
  readonly calls: string[] = [];
  fillStyle = '';
  strokeStyle = '';
  lineWidth = 0;
  lastArc: { angleFrom: number; angleTo: number } | null = null;

  save(): void {
    this.calls.push('save');
  }
  restore(): void {
    this.calls.push('restore');
  }
  beginPath(): void {
    this.calls.push('beginPath');
  }
  rect(): void {
    this.calls.push('rect');
  }
  clip(): void {
    this.calls.push('clip');
  }
  moveTo(): void {
    this.calls.push('moveTo');
  }
  lineTo(): void {
    this.calls.push('lineTo');
  }
  arc(_x: number, _y: number, _r: number, from: number, to: number): void {
    this.calls.push('arc');
    this.lastArc = { angleFrom: from, angleTo: to };
  }
  closePath(): void {
    this.calls.push('closePath');
  }
  fill(): void {
    this.calls.push('fill');
  }
  stroke(): void {
    this.calls.push('stroke');
  }
}

function asCtx(fake: FakeContext): CanvasRenderingContext2D {
  return fake as unknown as CanvasRenderingContext2D;
}

describe('Wischblende — Zeichnen', () => {
  it('bedeckt bei voller Abklingzeit den ganzen Kreis', () => {
    const ctx = new FakeContext();
    drawSweep(asCtx(ctx), 0, 0, 100, 100, 1);
    const arc = ctx.lastArc;
    expect(arc).not.toBeNull();
    expect(
      (arc as { angleTo: number; angleFrom: number }).angleTo -
        (arc as { angleFrom: number }).angleFrom,
    ).toBeCloseTo(Math.PI * 2);
  });

  it('zeichnet bei abgelaufener Abklingzeit gar nichts', () => {
    // Der Endzustand ist "weg", nicht "ein Grad übrig". Das ist der Fehler, vor
    // dem VERIFICATION § 4 warnt: eine Blende, die bei 359° stehen bleibt.
    const ctx = new FakeContext();
    drawSweep(asCtx(ctx), 0, 0, 100, 100, 0);
    expect(ctx.calls.length).toBe(0);
  });

  it('stellt den Zustand der Zeichenfläche wieder her', () => {
    const ctx = new FakeContext();
    drawSweep(asCtx(ctx), 0, 0, 100, 100, 0.5);
    expect(ctx.calls[0]).toBe('save');
    expect(ctx.calls[ctx.calls.length - 1]).toBe('restore');
    expect(ctx.calls.filter((c) => c === 'save').length).toBe(
      ctx.calls.filter((c) => c === 'restore').length,
    );
  });

  it('beschneidet auf den Platz, damit der Keil nicht übersteht', () => {
    const ctx = new FakeContext();
    drawSweep(asCtx(ctx), 10, 20, 100, 100, 0.5);
    expect(ctx.calls).toContain('clip');
  });

  it('begrenzt unsinnige Anteile', () => {
    const over = new FakeContext();
    drawSweep(asCtx(over), 0, 0, 100, 100, 5);
    const arcOver = over.lastArc as { angleFrom: number; angleTo: number };
    expect(arcOver.angleTo - arcOver.angleFrom).toBeCloseTo(Math.PI * 2);

    const under = new FakeContext();
    drawSweep(asCtx(under), 0, 0, 100, 100, -1);
    expect(under.calls.length).toBe(0);
  });
});
