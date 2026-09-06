import { describe, expect, it } from 'vitest';
import vectors from '../../sim/tests/fixtures/rounding-vectors.json';
import { computeDamage, NEUTRAL_FACTORS, roundLikeJavaScript } from '../../src/core/damage.js';

function fromBits(hex: string): number {
  const bytes = new ArrayBuffer(8);
  const view = new DataView(bytes);
  view.setBigUint64(0, BigInt(`0x${hex}`));
  return view.getFloat64(0);
}

describe('Gemeinsamer Rundungsvertrag mit Rust (#962)', () => {
  it('enthält beide Vorzeichen, -1.5 und dessen unmittelbare Nachbarn', () => {
    expect(vectors.schema).toBe(1);
    expect(vectors.cases.length).toBeGreaterThanOrEqual(71);
    for (const name of ['1.5-tie', '-1.5-below', '-1.5-tie', '-1.5-above']) {
      expect(vectors.cases.some((vector) => vector.name === name)).toBe(true);
    }
    expect(roundLikeJavaScript(-1.5)).toBe(-1);
  });

  for (const vector of vectors.cases) {
    it(`${vector.name}: ${vector.input} → ${vector.expected}`, () => {
      const actual = roundLikeJavaScript(fromBits(vector.inputBits));
      if (vector.expectedBits === null) {
        expect(Number.isNaN(actual)).toBe(true);
      } else {
        // Object.is (toBe) unterscheidet auch +0 und -0.
        expect(actual).toBe(fromBits(vector.expectedBits));
      }
    });
  }

  it('wendet dieselben Vektoren am zentralen Schadenspfad im u32-Bereich an', () => {
    let executed = 0;
    for (const vector of vectors.cases) {
      const input = fromBits(vector.inputBits);
      if (!(input >= 0) || input > 0xffff_ffff || vector.expectedBits === null) continue;
      const expected = fromBits(vector.expectedBits);
      // u32 hat keine negative Null. Der Browser behält sie bei -0 bei.
      expect(computeDamage(1, { ...NEUTRAL_FACTORS, essence: input })).toBe(expected);
      executed++;
    }
    expect(executed).toBeGreaterThanOrEqual(20);
  });

  it('erreicht einen negativen Zwischenwert in der echten Multiplikationskette', () => {
    // 1 * -1.5 * -1 = 1.5. Die Endwert-Zusicherung in Rust sieht den
    // vorübergehenden negativen Wert nicht; negative Faktoren sind API-Eingaben.
    expect(computeDamage(1, { ...NEUTRAL_FACTORS, essence: -1.5, talents: -1 })).toBe(2);
  });
});
