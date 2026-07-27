import { describe, expect, it } from 'vitest';
import { RingBufferF64 } from '../../src/core/ring.js';

describe('RingBufferF64', () => {
  it('gibt die Werte in chronologischer Reihenfolge zurück, solange Platz ist', () => {
    const ring = new RingBufferF64(8);
    for (let i = 1; i <= 5; i++) ring.push(i);

    expect(ring.length).toBe(5);
    expect(ring.toArray()).toEqual([1, 2, 3, 4, 5]);
    expect(ring.at(0)).toBe(1);
    expect(ring.at(4)).toBe(5);
  });

  it('überschreibt den ältesten Wert, wenn der Puffer voll ist', () => {
    const ring = new RingBufferF64(4);
    for (let i = 1; i <= 6; i++) ring.push(i);

    expect(ring.length).toBe(4);
    expect(ring.toArray()).toEqual([3, 4, 5, 6]);
    expect(ring.totalWritten).toBe(6);
    expect(ring.overwritten).toBe(2);
  });

  it('bleibt auch nach vielen Umläufen chronologisch', () => {
    const ring = new RingBufferF64(3);
    for (let i = 0; i < 1000; i++) ring.push(i);

    expect(ring.toArray()).toEqual([997, 998, 999]);
  });

  it('meldet Positionen außerhalb des gültigen Bereichs als NaN', () => {
    const ring = new RingBufferF64(4);
    ring.push(1);

    expect(Number.isNaN(ring.at(-1))).toBe(true);
    expect(Number.isNaN(ring.at(1))).toBe(true);
  });

  it('leert sich vollständig', () => {
    const ring = new RingBufferF64(4);
    for (let i = 0; i < 10; i++) ring.push(i);
    ring.clear();

    expect(ring.length).toBe(0);
    expect(ring.totalWritten).toBe(0);
    expect(ring.toArray()).toEqual([]);
  });

  it('weist unsinnige Größen zurück', () => {
    expect(() => new RingBufferF64(0)).toThrow(RangeError);
    expect(() => new RingBufferF64(-1)).toThrow(RangeError);
    expect(() => new RingBufferF64(2.5)).toThrow(RangeError);
  });

  it('fordert beim Schreiben keinen Speicher an', () => {
    // Der Nachweis über die Halde gehört in den Browser (VERIFICATION § 3.5).
    // Hier wird die Voraussetzung dafür geprüft: Der Puffer wächst nicht.
    const ring = new RingBufferF64(64);
    for (let i = 0; i < 100_000; i++) ring.push(i);

    expect(ring.capacity).toBe(64);
    expect(ring.length).toBe(64);
    expect(ring.at(63)).toBe(99_999);
  });
});
