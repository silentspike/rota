import { describe, expect, it } from 'vitest';
import { drawFixedRight, drawIntegerRight } from '../../src/render/numerals.js';

/**
 * Ersatz-Zeichenfläche: hält fest, welches Zeichen wohin gezeichnet wurde.
 * Der Vitest-Lauf braucht kein DOM (siehe `environment: 'node'`), und für die
 * Prüfung genügt die Folge der Zeichen.
 */
class FakeContext {
  readonly calls: { text: string; x: number; y: number }[] = [];

  fillText(text: string, x: number, y: number): void {
    this.calls.push({ text, x, y });
  }

  /** Die gezeichneten Zeichen von links nach rechts wieder zusammengesetzt. */
  rendered(): string {
    return [...this.calls]
      .sort((a, b) => a.x - b.x)
      .map((c) => c.text)
      .join('');
  }
}

function asCtx(fake: FakeContext): CanvasRenderingContext2D {
  return fake as unknown as CanvasRenderingContext2D;
}

const ADVANCE = 10;

describe('drawIntegerRight', () => {
  it('zeichnet eine einstellige Zahl', () => {
    const ctx = new FakeContext();
    drawIntegerRight(asCtx(ctx), 7, 100, 50, ADVANCE);

    expect(ctx.rendered()).toBe('7');
    expect(ctx.calls[0]?.x).toBe(90);
  });

  it('zeichnet null als einzelne Ziffer', () => {
    const ctx = new FakeContext();
    drawIntegerRight(asCtx(ctx), 0, 100, 50, ADVANCE);

    expect(ctx.rendered()).toBe('0');
  });

  it('zeichnet mehrstellige Zahlen rechtsbündig in der richtigen Reihenfolge', () => {
    const ctx = new FakeContext();
    const left = drawIntegerRight(asCtx(ctx), 4200, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('4200');
    expect(left).toBe(160);
    expect(ctx.calls.length).toBe(4);
  });

  it('zeichnet negative Zahlen mit Vorzeichen', () => {
    const ctx = new FakeContext();
    drawIntegerRight(asCtx(ctx), -19, 100, 50, ADVANCE);

    expect(ctx.rendered()).toBe('-19');
  });

  it('schneidet Nachkommastellen ab', () => {
    const ctx = new FakeContext();
    drawIntegerRight(asCtx(ctx), 12.9, 100, 50, ADVANCE);

    expect(ctx.rendered()).toBe('12');
  });

  it('kommt mit großen Zahlen zurecht', () => {
    const ctx = new FakeContext();
    drawIntegerRight(asCtx(ctx), 987654321, 1000, 50, ADVANCE);

    expect(ctx.rendered()).toBe('987654321');
  });
});

describe('drawFixedRight', () => {
  it('zeichnet zwei Nachkommastellen', () => {
    const ctx = new FakeContext();
    drawFixedRight(asCtx(ctx), 44.0, 2, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('44.00');
  });

  it('füllt fehlende Nachkommastellen mit Nullen', () => {
    const ctx = new FakeContext();
    drawFixedRight(asCtx(ctx), 3.5, 2, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('3.50');
  });

  it('rundet kaufmännisch auf die gewünschte Stelle', () => {
    const ctx = new FakeContext();
    drawFixedRight(asCtx(ctx), 1.996, 2, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('2.00');
  });

  it('zeichnet eine Nachkommastelle', () => {
    const ctx = new FakeContext();
    drawFixedRight(asCtx(ctx), 16.7, 1, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('16.7');
  });

  it('zeichnet Werte unter eins mit führender Null', () => {
    const ctx = new FakeContext();
    drawFixedRight(asCtx(ctx), 0.25, 2, 200, 50, ADVANCE);

    expect(ctx.rendered()).toBe('0.25');
  });
});
