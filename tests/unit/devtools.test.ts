import { describe, expect, it } from 'vitest';

/**
 * Beispieltest, der zugleich die Grundannahme des Determinismus prüft:
 * ganzzahlige Zeit darf über viele Schritte nicht driften (GAME-DESIGN 18.4).
 */
describe('ganzzahlige Zeit', () => {
  it('driftet über 10.000 Takte nicht', () => {
    const TICK_MS = 10;
    let t = 0;
    for (let i = 0; i < 10_000; i++) t += TICK_MS;
    expect(t).toBe(100_000);
    expect(Number.isInteger(t)).toBe(true);
  });

  it('rechnet Abklingzeiten exakt', () => {
    const GCD_MS = 1500;
    const casts = 40;
    expect(GCD_MS * casts).toBe(60_000);
  });
});
